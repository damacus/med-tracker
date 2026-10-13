use super::*;
use crate::models::{entities::api_app_token, identity::api_session};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Months;
use sea_orm::QueryTrait;
use sha2::{Digest, Sha256};

fn memberships(household_id: i64) -> sea_orm::sea_query::SelectStatement {
    membership::Entity::find()
        .select_only()
        .column(membership::Column::Id)
        .filter(membership::Column::HouseholdId.eq(household_id))
        .into_query()
}

fn summary(row: &api_app_token::Model) -> Value {
    json!({
        "id": row.id,
        "name": row.name,
        "last_used_at": row.last_used_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        "expires_at": row.expires_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        "revoked_at": row.revoked_at.map(|value| value.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
        "permissions_version": row.permissions_version,
    })
}

async fn token_for_account(
    tenant: &TenantTransaction,
    id: i64,
) -> Result<Option<api_app_token::Model>, OperationError> {
    api_app_token::Entity::find_by_id(id)
        .filter(api_app_token::Column::AccountId.eq(tenant.scope().actor.account_id))
        .filter(
            api_app_token::Column::HouseholdMembershipId
                .in_subquery(memberships(tenant.scope().household_id)),
        )
        .one(tenant.transaction())
        .await
        .map_err(|_| OperationError::Unavailable)
}

pub async fn index(tenant: &TenantTransaction) -> Result<Vec<Value>, OperationError> {
    access::recheck(tenant).await?;
    if !access::can_manage_household(tenant) {
        return Err(OperationError::Forbidden);
    }
    let rows = api_app_token::Entity::find()
        .filter(api_app_token::Column::AccountId.eq(tenant.scope().actor.account_id))
        .filter(
            api_app_token::Column::HouseholdMembershipId
                .in_subquery(memberships(tenant.scope().household_id)),
        )
        .order_by_asc(api_app_token::Column::Id)
        .all(tenant.transaction())
        .await
        .map_err(|_| OperationError::Unavailable)?;
    Ok(rows.iter().map(summary).collect())
}

pub async fn create(
    tenant: &TenantTransaction,
    attributes: &Value,
    provenance: &CredentialProvenance,
) -> Result<Value, OperationError> {
    let name = attributes
        .get("name")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| invalid("name", "can't be blank"))?
        .to_owned();
    let months = api_session::app_token_max_age_months(tenant.settings())
        .ok_or_else(|| invalid("expires_at", "configured maximum age is invalid"))?;
    let now = Utc::now().naive_utc();
    let maximum = now
        .checked_add_months(Months::new(months))
        .ok_or_else(|| invalid("expires_at", "configured maximum age is invalid"))?;
    let expiry = match attributes.get("expires_at") {
        None => maximum,
        Some(Value::String(value)) => DateTime::parse_from_rfc3339(value)
            .map(|value| value.with_timezone(&Utc).naive_utc())
            .map_err(|_| invalid("expires_at", "must be an RFC3339 timestamp"))?,
        Some(_) => return Err(invalid("expires_at", "must be an RFC3339 timestamp")),
    };
    if expiry <= now || expiry > maximum {
        return Err(invalid(
            "expires_at",
            "must be after issuance and within the configured maximum age",
        ));
    }
    let mut bytes = [0u8; 48];
    getrandom::fill(&mut bytes).map_err(|_| OperationError::Unavailable)?;
    let raw = format!("mt_app_{}", URL_SAFE_NO_PAD.encode(bytes));
    let digest = hex::encode(Sha256::digest(raw.as_bytes()));
    let row = api_app_token::ActiveModel {
        account_id: Set(tenant.scope().actor.account_id),
        household_membership_id: Set(tenant.membership().id),
        token_digest: Set(digest),
        permissions_version: Set(tenant.membership().permissions_version),
        name: Set(name.clone()),
        expires_at: Set(expiry),
        revoked_at: Set(None),
        last_used_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await
    .map_err(|_| OperationError::Unavailable)?;
    persistence::token_event(tenant, "created", &name, Some(provenance)).await?;
    let mut data = summary(&row);
    data["token"] = json!(raw);
    Ok(json!({"data": data}))
}

pub async fn revoke(
    tenant: &TenantTransaction,
    id: i64,
    provenance: &CredentialProvenance,
) -> Result<(), OperationError> {
    let Some(row) = token_for_account(tenant, id).await? else {
        return Err(OperationError::NotFound);
    };
    if row.revoked_at.is_none() {
        let now = Utc::now().naive_utc();
        let mut active: api_app_token::ActiveModel = row.clone().into();
        active.revoked_at = Set(Some(now));
        active.updated_at = Set(now);
        active
            .update(tenant.transaction())
            .await
            .map_err(|_| OperationError::Unavailable)?;
        persistence::token_event(tenant, "revoked", &row.name, Some(provenance)).await?;
    }
    Ok(())
}
