use crate::entities::{grant, person, user};
use crate::medication_management::{
    error_response, finish, finish_with_request_id, record_version, request_context,
};
use crate::read_entities::notification_preference;
use crate::sync_events::{lock_household, record_change, SyncRecord};
use crate::{database_error, representation_etag, ApiError, AppState, AuthContext};
use axum::extract::{rejection::JsonRejection, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::{NaiveTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseTransaction, EntityTrait, IntoActiveModel,
    QueryFilter, Set,
};
use serde_json::{json, Map, Value};
use uuid::Uuid;

const CONTROLLER: &str = "api/v1/notification_preferences";
const POLICY: &str = "NotificationPreferencePolicy";

#[derive(Default)]
struct Attributes {
    enabled: Option<bool>,
    dose_due_enabled: Option<bool>,
    missed_dose_enabled: Option<bool>,
    low_stock_enabled: Option<bool>,
    private_text_enabled: Option<bool>,
    morning_time: Option<Option<NaiveTime>>,
    afternoon_time: Option<Option<NaiveTime>>,
    evening_time: Option<Option<NaiveTime>>,
    night_time: Option<Option<NaiveTime>>,
}

impl Attributes {
    fn parse(value: &Value) -> Option<Self> {
        let attributes = value.as_object()?;
        if attributes.is_empty()
            || attributes.keys().any(|key| {
                !matches!(
                    key.as_str(),
                    "enabled"
                        | "dose_due_enabled"
                        | "missed_dose_enabled"
                        | "low_stock_enabled"
                        | "private_text_enabled"
                        | "morning_time"
                        | "afternoon_time"
                        | "evening_time"
                        | "night_time"
                )
            })
        {
            return None;
        }
        Some(Self {
            enabled: boolean(attributes, "enabled")?,
            dose_due_enabled: boolean(attributes, "dose_due_enabled")?,
            missed_dose_enabled: boolean(attributes, "missed_dose_enabled")?,
            low_stock_enabled: boolean(attributes, "low_stock_enabled")?,
            private_text_enabled: boolean(attributes, "private_text_enabled")?,
            morning_time: time(attributes, "morning_time")?,
            afternoon_time: time(attributes, "afternoon_time")?,
            evening_time: time(attributes, "evening_time")?,
            night_time: time(attributes, "night_time")?,
        })
    }

    fn matches(&self, row: &notification_preference::Model) -> bool {
        self.enabled.is_none_or(|value| value == row.enabled)
            && self
                .dose_due_enabled
                .is_none_or(|value| value == row.dose_due_enabled)
            && self
                .missed_dose_enabled
                .is_none_or(|value| value == row.missed_dose_enabled)
            && self
                .low_stock_enabled
                .is_none_or(|value| value == row.low_stock_enabled)
            && self
                .private_text_enabled
                .is_none_or(|value| value == row.private_text_enabled)
            && self
                .morning_time
                .is_none_or(|value| value == row.morning_time)
            && self
                .afternoon_time
                .is_none_or(|value| value == row.afternoon_time)
            && self
                .evening_time
                .is_none_or(|value| value == row.evening_time)
            && self.night_time.is_none_or(|value| value == row.night_time)
    }

    fn apply(self, row: &mut notification_preference::ActiveModel) {
        if let Some(value) = self.enabled {
            row.enabled = Set(value);
        }
        if let Some(value) = self.dose_due_enabled {
            row.dose_due_enabled = Set(value);
        }
        if let Some(value) = self.missed_dose_enabled {
            row.missed_dose_enabled = Set(value);
        }
        if let Some(value) = self.low_stock_enabled {
            row.low_stock_enabled = Set(value);
        }
        if let Some(value) = self.private_text_enabled {
            row.private_text_enabled = Set(value);
        }
        if let Some(value) = self.morning_time {
            row.morning_time = Set(value);
        }
        if let Some(value) = self.afternoon_time {
            row.afternoon_time = Set(value);
        }
        if let Some(value) = self.evening_time {
            row.evening_time = Set(value);
        }
        if let Some(value) = self.night_time {
            row.night_time = Set(value);
        }
    }
}

fn boolean(attributes: &Map<String, Value>, key: &str) -> Option<Option<bool>> {
    attributes
        .get(key)
        .map(Value::as_bool)
        .map_or(Some(None), |value| value.map(Some))
}

fn time(attributes: &Map<String, Value>, key: &str) -> Option<Option<Option<NaiveTime>>> {
    let Some(value) = attributes.get(key) else {
        return Some(None);
    };
    if value.is_null() {
        return Some(Some(None));
    }
    let text = value.as_str()?;
    let bytes = text.as_bytes();
    let valid_shape = (bytes.len() == 5 || bytes.len() == 8)
        && bytes[2] == b':'
        && (bytes.len() == 5 || bytes[5] == b':')
        && bytes.iter().enumerate().all(|(index, byte)| {
            index == 2 || (index == 5 && bytes.len() == 8) || byte.is_ascii_digit()
        });
    if !valid_shape || (bytes.len() == 8 && &bytes[6..8] > b"59".as_slice()) {
        return None;
    }
    let format = if bytes.len() == 5 {
        "%H:%M"
    } else {
        "%H:%M:%S"
    };
    NaiveTime::parse_from_str(text, format)
        .ok()
        .map(|value| Some(Some(value)))
}

fn value(row: &notification_preference::Model, owner: &person::Model) -> Value {
    let formatted = |time: Option<NaiveTime>| time.map(|time| time.format("%H:%M:%S").to_string());
    json!({
        "id": row.id,
        "portable_id": row.portable_id,
        "person_id": row.person_id,
        "person_portable_id": owner.portable_id,
        "enabled": row.enabled,
        "dose_due_enabled": row.dose_due_enabled,
        "missed_dose_enabled": row.missed_dose_enabled,
        "low_stock_enabled": row.low_stock_enabled,
        "private_text_enabled": row.private_text_enabled,
        "morning_time": formatted(row.morning_time),
        "afternoon_time": formatted(row.afternoon_time),
        "evening_time": formatted(row.evening_time),
        "night_time": formatted(row.night_time),
        "updated_at": row.updated_at.and_utc().to_rfc3339(),
    })
}

fn representation(row: &notification_preference::Model, owner: &person::Model) -> (Value, String) {
    let body = json!({"data": value(row, owner)});
    let etag = representation_etag(&body);
    (body, etag)
}

async fn failure(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    action: &str,
    status: StatusCode,
    code: &str,
    message: &str,
) -> Result<Response, ApiError> {
    error_response(
        db, context, method, CONTROLLER, POLICY, action, status, code, message, None,
    )
    .await
}

async fn context_for_person(
    state: &AppState,
    headers: &HeaderMap,
    household_id: i64,
    method: &str,
    action: &str,
    manage: bool,
) -> Result<Result<(DatabaseTransaction, AuthContext, person::Model), Response>, ApiError> {
    let (db, context) = request_context(state, headers, household_id).await?;
    let user = user::Entity::find_by_id(context.user_id)
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    let owner = person::Entity::find_by_id(user.person_id)
        .one(&db)
        .await
        .map_err(database_error)?;
    let Some(owner) = owner.filter(|person| person.household_id == household_id) else {
        let status = if manage {
            StatusCode::FORBIDDEN
        } else {
            StatusCode::NOT_FOUND
        };
        let (code, message) = if manage {
            (
                "forbidden",
                "You are not authorized to perform this action.",
            )
        } else {
            ("not_found", "Resource not found")
        };
        return Ok(Err(failure(
            db, &context, method, action, status, code, message,
        )
        .await?));
    };
    if manage {
        lock_household(&db, household_id).await?;
    }
    let mut query = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(grant::Column::PersonId.eq(owner.id))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
        );
    query = if manage {
        query.filter(grant::Column::AccessLevel.eq("manage"))
    } else {
        query.filter(grant::Column::AccessLevel.is_in(["view", "record", "manage"]))
    };
    if query.one(&db).await.map_err(database_error)?.is_none() {
        let status = if manage {
            StatusCode::FORBIDDEN
        } else {
            StatusCode::NOT_FOUND
        };
        let (code, message) = if manage {
            (
                "forbidden",
                "You are not authorized to perform this action.",
            )
        } else {
            ("not_found", "Resource not found")
        };
        return Ok(Err(failure(
            db, &context, method, action, status, code, message,
        )
        .await?));
    }
    Ok(Ok((db, context, owner)))
}

pub(super) async fn show(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context, owner) =
        match context_for_person(&state, &headers, household_id, "GET", "show", false).await? {
            Ok(value) => value,
            Err(response) => return Ok(response),
        };
    let row = notification_preference::Entity::find()
        .filter(notification_preference::Column::HouseholdId.eq(household_id))
        .filter(notification_preference::Column::PersonId.eq(owner.id))
        .one(&db)
        .await
        .map_err(database_error)?;
    let Some(row) = row else {
        return failure(
            db,
            &context,
            "GET",
            "show",
            StatusCode::NOT_FOUND,
            "not_found",
            "Resource not found",
        )
        .await;
    };
    let (body, etag) = representation(&row, &owner);
    finish(
        db,
        &context,
        "GET",
        CONTROLLER,
        POLICY,
        "show",
        StatusCode::OK,
        true,
        body,
        Some(&etag),
    )
    .await
}

pub(super) async fn patch(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    update(state, household_id, headers, payload, "PATCH").await
}

pub(super) async fn put(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    update(state, household_id, headers, payload, "PUT").await
}

async fn update(
    state: AppState,
    household_id: i64,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
    method: &str,
) -> Result<Response, ApiError> {
    let (db, context, owner) =
        match context_for_person(&state, &headers, household_id, method, "update", true).await? {
            Ok(value) => value,
            Err(response) => return Ok(response),
        };
    let Json(body) = match payload {
        Ok(body) => body,
        Err(_) => {
            return failure(
                db,
                &context,
                method,
                "update",
                StatusCode::BAD_REQUEST,
                "bad_request",
                "Invalid JSON request",
            )
            .await
        }
    };
    let Some(outer) = body.as_object() else {
        return failure(
            db,
            &context,
            method,
            "update",
            StatusCode::BAD_REQUEST,
            "bad_request",
            "notification_preference is required",
        )
        .await;
    };
    let Some(inner) = outer.get("notification_preference") else {
        return failure(
            db,
            &context,
            method,
            "update",
            StatusCode::BAD_REQUEST,
            "bad_request",
            "notification_preference is required",
        )
        .await;
    };
    let Some(attributes) = Attributes::parse(inner).filter(|_| outer.len() == 1) else {
        return error_response(
            db,
            &context,
            method,
            CONTROLLER,
            POLICY,
            "update",
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Validation failed",
            Some(json!({"notification_preference": ["is invalid"]})),
        )
        .await;
    };
    let previous = notification_preference::Entity::find()
        .filter(notification_preference::Column::HouseholdId.eq(household_id))
        .filter(notification_preference::Column::PersonId.eq(owner.id))
        .one(&db)
        .await
        .map_err(database_error)?;
    if let Some(previous) = previous.as_ref().filter(|row| attributes.matches(row)) {
        let (body, etag) = representation(previous, &owner);
        return finish(
            db,
            &context,
            method,
            CONTROLLER,
            POLICY,
            "update",
            StatusCode::OK,
            true,
            body,
            Some(&etag),
        )
        .await;
    }
    let now = Utc::now().naive_utc();
    let mut active = if let Some(row) = previous.clone() {
        row.into_active_model()
    } else {
        notification_preference::ActiveModel {
            household_id: Set(household_id),
            person_id: Set(owner.id),
            created_at: Set(now),
            ..Default::default()
        }
    };
    active.updated_at = Set(now);
    attributes.apply(&mut active);
    let row = if previous.is_some() {
        active.update(&db).await
    } else {
        active.insert(&db).await
    }
    .map_err(database_error)?;
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "NotificationPreference",
        row.id,
        if previous.is_some() {
            "update"
        } else {
            "create"
        },
        previous.as_ref().map(|row| value(row, &owner)),
        Some(value(&row, &owner)),
    )
    .await?;
    record_change(
        &db,
        &context,
        &request_id,
        SyncRecord {
            record_type: "NotificationPreference",
            record_id: row.id,
            portable_id: &row.portable_id,
            action: if previous.is_some() {
                "update"
            } else {
                "create"
            },
            person_portable_id: Some(&owner.portable_id),
        },
    )
    .await?;
    let (body, etag) = representation(&row, &owner);
    finish_with_request_id(
        db,
        &context,
        &request_id,
        method,
        CONTROLLER,
        POLICY,
        "update",
        StatusCode::OK,
        true,
        body,
        Some(&etag),
    )
    .await
}
