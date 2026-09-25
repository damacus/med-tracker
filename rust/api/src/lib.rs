mod entities;
mod medication_forecast;

use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use entities::{
    account, account_lockout, api_session, grant, household, location, medication, membership,
    person, person_medication, schedule, user,
};
use sea_orm::{
    ColumnTrait, Condition, ConnectOptions, ConnectionTrait, Database, DatabaseConnection,
    DatabaseTransaction, DbBackend, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, QueryTrait, Statement, TransactionTrait,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Clone)]
pub struct AppState {
    db: DatabaseConnection,
}

pub async fn connect(url: &str) -> Result<AppState, sea_orm::DbErr> {
    let mut options = ConnectOptions::new(url);
    options.max_connections(4).min_connections(1);
    options.connect_timeout(Duration::from_secs(5));
    options.acquire_timeout(Duration::from_secs(5));
    options.sqlx_logging(false);
    let db = Database::connect(options).await?;
    let transaction = db.begin().await?;
    restricted_role(&transaction).await?;
    let role = transaction
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT current_role AS role, row_security_active('medications'::regclass) AS rls_active",
        ))
        .await?
        .ok_or_else(|| sea_orm::DbErr::Custom("database role check returned no row".to_owned()))?;
    let name: String = role.try_get("", "role")?;
    let rls_active: bool = role.try_get("", "rls_active")?;
    if name != "med_tracker_app" || !rls_active {
        return Err(sea_orm::DbErr::Custom(
            "medication RLS must be active under med_tracker_app".to_owned(),
        ));
    }
    transaction.rollback().await?;
    Ok(AppState { db })
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/up", get(|| async { StatusCode::OK }))
        .route("/api/v1/households/{household_id}/medications", get(index))
        .route(
            "/api/v1/households/{household_id}/medications/{id}",
            get(show),
        )
        .with_state(state)
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: &'static str,
}

impl ApiError {
    fn unauthorized() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "unauthorized",
            message: "Authentication required",
        }
    }

    fn forbidden() -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code: "forbidden",
            message: "You are not authorized to perform this action.",
        }
    }

    fn not_found() -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "not_found",
            message: "Record not found",
        }
    }

    fn internal() -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal_error",
            message: "Internal server error",
        }
    }

    fn invalid_filter() -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "unprocessable_content",
            message: "updated_since must be ISO8601",
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({"error": {"code": self.code, "message": self.message, "request_id": uuid::Uuid::new_v4().to_string()}}))).into_response()
    }
}

fn database_error(error: sea_orm::DbErr) -> ApiError {
    eprintln!("medication read database error: {error}");
    ApiError::internal()
}

async fn restricted_role(db: &DatabaseTransaction) -> Result<(), sea_orm::DbErr> {
    db.execute_raw(Statement::from_string(
        DbBackend::Postgres,
        "SET LOCAL ROLE med_tracker_app",
    ))
    .await?;
    Ok(())
}

async fn tenant_setting(
    db: &DatabaseTransaction,
    name: &str,
    value: i64,
) -> Result<(), sea_orm::DbErr> {
    db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT set_config($1, $2, true)",
        [name.into(), value.to_string().into()],
    ))
    .await?;
    Ok(())
}

struct AuthContext {
    membership: membership::Model,
}

async fn authenticate(
    db: &DatabaseTransaction,
    headers: &HeaderMap,
    household_id: i64,
) -> Result<AuthContext, ApiError> {
    restricted_role(db).await.map_err(database_error)?;
    let token = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| !value.is_empty())
        .ok_or_else(ApiError::unauthorized)?;
    let digest = format!("{:x}", Sha256::digest(token.as_bytes()));
    let session = api_session::Entity::find()
        .filter(api_session::Column::AccessTokenDigest.eq(digest))
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    if session.revoked_at.is_some() || session.access_expires_at <= Utc::now().naive_utc() {
        return Err(ApiError::unauthorized());
    }
    let account = account::Entity::find_by_id(session.account_id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    if account.status != 2 {
        return Err(ApiError::unauthorized());
    }
    let lockout = account_lockout::Entity::find_by_id(account.id)
        .one(db)
        .await
        .map_err(database_error)?;
    if lockout.is_some_and(|lockout| lockout.deadline > Utc::now().naive_utc()) {
        return Err(ApiError::unauthorized());
    }
    tenant_setting(db, "med_tracker.current_account_id", account.id)
        .await
        .map_err(database_error)?;
    let household = household::Entity::find_by_id(household_id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    if household.status != "active" || household.lifecycle_state != "active" {
        return Err(ApiError::forbidden());
    }
    let membership_id = session
        .household_membership_id
        .ok_or_else(ApiError::unauthorized)?;
    let membership = membership::Entity::find_by_id(membership_id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    if membership.account_id != account.id
        || membership.status != "active"
        || membership.revoked_at.is_some()
        || membership.permissions_version != session.permissions_version
    {
        return Err(ApiError::unauthorized());
    }
    if membership.household_id != household_id {
        return Err(ApiError::forbidden());
    }
    tenant_setting(db, "med_tracker.current_household_id", household_id)
        .await
        .map_err(database_error)?;
    tenant_setting(db, "med_tracker.current_membership_id", membership.id)
        .await
        .map_err(database_error)?;
    let person = person::Entity::find()
        .filter(person::Column::AccountId.eq(account.id))
        .order_by_asc(person::Column::Id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    let user = user::Entity::find()
        .filter(user::Column::PersonId.eq(person.id))
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    if !user.active {
        return Err(ApiError::unauthorized());
    }
    Ok(AuthContext { membership })
}

fn granted_people(membership: &membership::Model) -> sea_orm::sea_query::SelectStatement {
    grant::Entity::find()
        .select_only()
        .column(grant::Column::PersonId)
        .filter(grant::Column::HouseholdId.eq(membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(membership.id))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(grant::Column::AccessLevel.is_in(["view", "record", "manage"]))
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
        )
        .into_query()
}

fn scope(household_id: i64, membership: &membership::Model) -> sea_orm::Select<medication::Entity> {
    let query = medication::Entity::find().filter(medication::Column::HouseholdId.eq(household_id));
    if membership.role == "owner" || membership.role == "administrator" {
        return query;
    }
    let granted_schedules = schedule::Entity::find()
        .select_only()
        .column(schedule::Column::MedicationId)
        .filter(schedule::Column::HouseholdId.eq(household_id))
        .filter(schedule::Column::PersonId.in_subquery(granted_people(membership)))
        .into_query();
    let granted_assignments = person_medication::Entity::find()
        .select_only()
        .column(person_medication::Column::MedicationId)
        .filter(person_medication::Column::HouseholdId.eq(household_id))
        .filter(person_medication::Column::PersonId.in_subquery(granted_people(membership)))
        .into_query();
    let linked_schedules = schedule::Entity::find()
        .select_only()
        .column(schedule::Column::MedicationId)
        .filter(schedule::Column::HouseholdId.eq(household_id))
        .into_query();
    let linked_assignments = person_medication::Entity::find()
        .select_only()
        .column(person_medication::Column::MedicationId)
        .filter(person_medication::Column::HouseholdId.eq(household_id))
        .into_query();
    query.filter(
        Condition::any()
            .add(medication::Column::Id.in_subquery(granted_schedules))
            .add(medication::Column::Id.in_subquery(granted_assignments))
            .add(
                Condition::all()
                    .add(medication::Column::CreatedByMembershipId.eq(membership.id))
                    .add(medication::Column::Id.not_in_subquery(linked_schedules))
                    .add(medication::Column::Id.not_in_subquery(linked_assignments)),
            ),
    )
}

#[derive(Deserialize)]
struct Pagination {
    page: Option<i64>,
    per_page: Option<i64>,
    updated_since: Option<String>,
}

async fn index(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    Query(pagination): Query<Pagination>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let db = state.db.begin().await.map_err(database_error)?;
    let context = authenticate(&db, &headers, household_id).await?;
    let page = pagination.page.unwrap_or(1).max(1);
    let per_page = pagination.per_page.unwrap_or(20).clamp(1, 100);
    let updated_since = pagination
        .updated_since
        .filter(|value| !value.is_empty())
        .map(|value| DateTime::parse_from_rfc3339(&value).map_err(|_| ApiError::invalid_filter()))
        .transpose()?
        .map(|value| value.naive_utc());
    let mut query = scope(household_id, &context.membership);
    if let Some(updated_since) = updated_since {
        query = query.filter(medication::Column::UpdatedAt.gte(updated_since));
    }
    let total = query.clone().count(&db).await.map_err(database_error)?;
    let records = query
        .order_by_asc(medication::Column::Id)
        .limit(per_page as u64)
        .offset(((page - 1) * per_page) as u64)
        .all(&db)
        .await
        .map_err(database_error)?;
    let data = serialize_many(&db, records).await?;
    db.commit().await.map_err(database_error)?;
    Ok(Json(
        json!({"data": data, "meta": {"page": page, "per_page": per_page, "total_count": total}}),
    ))
}

async fn show(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let db = state.db.begin().await.map_err(database_error)?;
    let context = authenticate(&db, &headers, household_id).await?;
    let query = scope(household_id, &context.membership);
    let query = match id.parse::<i64>() {
        Ok(id) => query.filter(medication::Column::Id.eq(id)),
        Err(_) => query.filter(medication::Column::PortableId.eq(id)),
    };
    let record = query
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    let data = serialize_many(&db, vec![record]).await?.remove(0);
    let body = json!({"data": data});
    let etag = representation_etag(&body);
    if if_none_match_matches(&headers, &etag) {
        db.commit().await.map_err(database_error)?;
        let mut response = StatusCode::NOT_MODIFIED.into_response();
        response.headers_mut().insert(
            header::ETAG,
            HeaderValue::from_str(&etag).map_err(|_| ApiError::internal())?,
        );
        return Ok(response);
    }
    db.commit().await.map_err(database_error)?;
    let mut response = Json(body).into_response();
    response.headers_mut().insert(
        header::ETAG,
        HeaderValue::from_str(&etag).map_err(|_| ApiError::internal())?,
    );
    Ok(response)
}

fn if_none_match_matches(headers: &HeaderMap, etag: &str) -> bool {
    headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value.split(',').any(|tag| {
                let tag = tag.trim();
                tag == "*" || tag.strip_prefix("W/").unwrap_or(tag) == etag
            })
        })
}

fn representation_etag(body: &Value) -> String {
    format!(
        "\"{:x}\"",
        Sha256::digest(serde_json::to_vec(body).expect("JSON value must serialize"))
    )
}

async fn serialize_many(
    db: &DatabaseTransaction,
    records: Vec<medication::Model>,
) -> Result<Vec<Value>, ApiError> {
    let location_ids: Vec<i64> = records.iter().map(|record| record.location_id).collect();
    let locations: HashMap<i64, String> = if location_ids.is_empty() {
        HashMap::new()
    } else {
        location::Entity::find()
            .filter(location::Column::Id.is_in(location_ids))
            .all(db)
            .await
            .map_err(database_error)?
            .into_iter()
            .map(|location| (location.id, location.portable_id))
            .collect()
    };
    let forecasts = medication_forecast::for_medications(db, &records)
        .await
        .map_err(database_error)?;
    Ok(records.into_iter().map(|record| {
        let forecast = forecasts.get(&record.id).copied().unwrap_or_default();
        let display_name = record.friendly_name.as_ref().filter(|name| !name.is_empty()).or(record.name.as_ref());
        let low_stock = record.current_supply.is_some_and(|supply| supply <= record.reorder_threshold);
        let out_of_stock = record.current_supply.is_some_and(|supply| supply <= 0.into());
        json!({
            "id": record.id,
            "portable_id": record.portable_id,
            "name": record.name,
            "display_name": display_name,
            "category": record.category,
            "description": record.description,
            "dose_amount": record.dose_amount.map(|amount| decimal_string(amount.to_string())),
            "dose_unit": record.dose_unit,
            "current_supply": record.current_supply.map(|value| decimal_string(value.to_string())),
            "reorder_threshold": decimal_string(record.reorder_threshold.to_string()),
            "reorder_status": record.reorder_status.map(|value| if value == 1 { "ordered" } else { "received" }),
            "location_id": record.location_id,
            "location_portable_id": locations.get(&record.location_id),
            "updated_at": record.updated_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            "low_stock": low_stock,
            "out_of_stock": out_of_stock,
            "days_until_low_stock": forecast.days_until_low_stock,
            "days_until_out_of_stock": forecast.days_until_out_of_stock
        })
    }).collect())
}

fn decimal_string(value: String) -> String {
    if let Some((whole, fraction)) = value.split_once('.') {
        let fraction = fraction.trim_end_matches('0');
        if fraction.is_empty() {
            format!("{whole}.0")
        } else {
            format!("{whole}.{fraction}")
        }
    } else {
        format!("{value}.0")
    }
}

#[cfg(test)]
mod tests {
    use super::{decimal_string, representation_etag};
    use serde_json::json;

    #[test]
    fn etag_changes_with_forecast_and_location_in_the_response() {
        let initial =
            json!({"data": {"id": 1, "location_portable_id": "A", "days_until_low_stock": null}});
        let changed_forecast =
            json!({"data": {"id": 1, "location_portable_id": "A", "days_until_low_stock": 3}});
        let changed_location =
            json!({"data": {"id": 1, "location_portable_id": "B", "days_until_low_stock": null}});
        assert_ne!(
            representation_etag(&initial),
            representation_etag(&changed_forecast)
        );
        assert_ne!(
            representation_etag(&initial),
            representation_etag(&changed_location)
        );
        assert_eq!(representation_etag(&initial), representation_etag(&initial));
    }

    #[test]
    fn decimal_strings_keep_integer_digits_and_one_fraction_digit() {
        assert_eq!(decimal_string("80.00".to_owned()), "80.0");
        assert_eq!(decimal_string("10.00".to_owned()), "10.0");
        assert_eq!(decimal_string("0.00".to_owned()), "0.0");
        assert_eq!(decimal_string("80".to_owned()), "80.0");
        assert_eq!(decimal_string("2.125".to_owned()), "2.125");
    }
}
