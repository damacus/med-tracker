use super::*;
use crate::models::{entities::api_app_token, identity::api_session};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Months;
use sea_orm::QueryTrait;
use serde::{Serialize, Serializer};
use sha2::{Digest, Sha256};

pub enum TextInput {
    Missing,
    Text(String),
    Invalid,
}

pub struct CreateInput {
    pub name: TextInput,
    pub expires_at: TextInput,
}

struct CreateCommand {
    name: String,
    expires_at: chrono::NaiveDateTime,
    issued_at: chrono::NaiveDateTime,
}

#[derive(Serialize)]
pub struct Summary {
    pub id: i64,
    pub name: String,
    #[serde(serialize_with = "serialize_second")]
    pub last_used_at: chrono::NaiveDateTime,
    #[serde(serialize_with = "serialize_second")]
    pub expires_at: chrono::NaiveDateTime,
    #[serde(serialize_with = "serialize_optional_second")]
    pub revoked_at: Option<chrono::NaiveDateTime>,
    pub permissions_version: i32,
}

#[derive(Serialize)]
pub struct IssuedToken {
    #[serde(flatten)]
    pub summary: Summary,
    pub token: String,
}

fn serialize_second<S>(value: &chrono::NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.format("%Y-%m-%dT%H:%M:%SZ").to_string())
}

fn serialize_optional_second<S>(
    value: &Option<chrono::NaiveDateTime>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(value) => serialize_second(value, serializer),
        None => serializer.serialize_none(),
    }
}

fn memberships(household_id: i64) -> sea_orm::sea_query::SelectStatement {
    membership::Entity::find()
        .select_only()
        .column(membership::Column::Id)
        .filter(membership::Column::HouseholdId.eq(household_id))
        .into_query()
}

fn summary(row: &api_app_token::Model) -> Summary {
    Summary {
        id: row.id,
        name: row.name.clone(),
        last_used_at: row.last_used_at,
        expires_at: row.expires_at,
        revoked_at: row.revoked_at,
        permissions_version: row.permissions_version,
    }
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
        .map_err(OperationError::from)
}

pub async fn index(tenant: &TenantTransaction) -> Result<Vec<Summary>, OperationError> {
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
        .map_err(OperationError::from)?;
    Ok(rows.iter().map(summary).collect())
}

fn validate(input: CreateInput, settings: Option<&Value>) -> Result<CreateCommand, OperationError> {
    let name = match input.name {
        TextInput::Text(name) if !name.trim().is_empty() => name,
        TextInput::Missing | TextInput::Text(_) | TextInput::Invalid => {
            return Err(invalid("name", "can't be blank"));
        }
    };
    let months = api_session::app_token_max_age_months(settings)
        .ok_or_else(|| invalid("expires_at", "configured maximum age is invalid"))?;
    let now = Utc::now().naive_utc();
    let maximum = now
        .checked_add_months(Months::new(months))
        .ok_or_else(|| invalid("expires_at", "configured maximum age is invalid"))?;
    let expiry = match input.expires_at {
        TextInput::Missing => maximum,
        TextInput::Text(value) => DateTime::parse_from_rfc3339(&value)
            .map(|value| value.with_timezone(&Utc).naive_utc())
            .map_err(|_| invalid("expires_at", "must be an RFC3339 timestamp"))?,
        TextInput::Invalid => return Err(invalid("expires_at", "must be an RFC3339 timestamp")),
    };
    if expiry <= now || expiry > maximum {
        return Err(invalid(
            "expires_at",
            "must be after issuance and within the configured maximum age",
        ));
    }
    Ok(CreateCommand {
        name,
        expires_at: expiry,
        issued_at: now,
    })
}

pub async fn create(
    tenant: &TenantTransaction,
    input: CreateInput,
    provenance: &CredentialProvenance,
) -> Result<IssuedToken, OperationError> {
    let command = validate(input, tenant.settings())?;
    issue(tenant, command, provenance).await
}

async fn issue(
    tenant: &TenantTransaction,
    command: CreateCommand,
    provenance: &CredentialProvenance,
) -> Result<IssuedToken, OperationError> {
    let mut bytes = [0u8; 48];
    getrandom::fill(&mut bytes).map_err(|_| OperationError::Unavailable)?;
    let raw = format!("mt_app_{}", URL_SAFE_NO_PAD.encode(bytes));
    let digest = hex::encode(Sha256::digest(raw.as_bytes()));
    let row = api_app_token::ActiveModel {
        account_id: Set(tenant.scope().actor.account_id),
        household_membership_id: Set(tenant.membership().id),
        token_digest: Set(digest),
        permissions_version: Set(tenant.membership().permissions_version),
        name: Set(command.name.clone()),
        expires_at: Set(command.expires_at),
        revoked_at: Set(None),
        last_used_at: Set(command.issued_at),
        created_at: Set(command.issued_at),
        updated_at: Set(command.issued_at),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await
    .map_err(OperationError::from)?;
    persistence::token_event(tenant, "created", &command.name, Some(provenance)).await?;
    Ok(IssuedToken {
        summary: summary(&row),
        token: raw,
    })
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
            .map_err(OperationError::from)?;
        persistence::token_event(tenant, "revoked", &row.name, Some(provenance)).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_creation_preserves_name_config_and_expiry_validation_order() {
        let invalid_config = json!({"api":{"app_token_max_age_months":0}});
        let missing_name = validate(
            CreateInput {
                name: TextInput::Missing,
                expires_at: TextInput::Invalid,
            },
            Some(&invalid_config),
        )
        .err()
        .unwrap();
        assert_eq!(missing_name, invalid("name", "can't be blank"));

        let invalid_maximum = validate(
            CreateInput {
                name: TextInput::Text("Device".into()),
                expires_at: TextInput::Invalid,
            },
            Some(&invalid_config),
        )
        .err()
        .unwrap();
        assert_eq!(
            invalid_maximum,
            invalid("expires_at", "configured maximum age is invalid")
        );

        let invalid_expiry = validate(
            CreateInput {
                name: TextInput::Text("Device".into()),
                expires_at: TextInput::Invalid,
            },
            None,
        )
        .err()
        .unwrap();
        assert_eq!(
            invalid_expiry,
            invalid("expires_at", "must be an RFC3339 timestamp")
        );

        let requested = Utc::now() + chrono::Duration::days(1);
        let with_offset = requested.with_timezone(&chrono::FixedOffset::east_opt(7200).unwrap());
        let command = validate(
            CreateInput {
                name: TextInput::Text("  Device  ".into()),
                expires_at: TextInput::Text(with_offset.to_rfc3339()),
            },
            None,
        )
        .ok()
        .unwrap();
        assert_eq!(command.name, "  Device  ");
        assert_eq!(command.expires_at, requested.naive_utc());
    }

    #[test]
    fn typed_summary_keeps_utc_second_spelling_and_explicit_null() {
        let timestamp = chrono::DateTime::parse_from_rfc3339("2026-10-23T12:34:56.123456Z")
            .unwrap()
            .naive_utc();
        let row = Summary {
            id: 10,
            name: "Device".into(),
            last_used_at: timestamp,
            expires_at: timestamp,
            revoked_at: None,
            permissions_version: 3,
        };
        let wire = serde_json::to_value(row).unwrap();
        assert_eq!(wire["last_used_at"], "2026-10-23T12:34:56Z");
        assert_eq!(wire["expires_at"], "2026-10-23T12:34:56Z");
        assert_eq!(wire["revoked_at"], Value::Null);
    }
}
