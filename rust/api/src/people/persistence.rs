use crate::database_error;
use crate::entities::grant;
use crate::entities::security_audit_event;
use crate::read_entities::person;
use crate::read_resources::serialize_people;
use crate::representation_etag;
use crate::ApiError;
use crate::AuthContext;
use crate::CredentialKind;
use chrono::Utc;
use sea_orm::ActiveModelTrait;
use sea_orm::DatabaseTransaction;
use sea_orm::Set;
use serde_json::json;
use serde_json::Value;

pub(super) fn snapshot(record: &person::Model) -> Value {
    json!({"id": record.id, "account_id": record.account_id, "household_id": record.household_id, "portable_id": record.portable_id, "name": record.name, "email": record.email, "date_of_birth": record.date_of_birth, "person_type": record.person_type, "has_capacity": record.has_capacity, "created_at": record.created_at, "updated_at": record.updated_at})
}

pub(super) fn grant_state(record: &grant::Model) -> Value {
    json!({
        "household_membership_id": record.household_membership_id,
        "person_id": record.person_id,
        "access_level": record.access_level,
        "relationship_type": record.relationship_type,
        "expires_at": record.expires_at,
        "revoked_at": record.revoked_at,
        "carer_relationship_id": record.carer_relationship_id
    })
}

pub(super) async fn record_grant_event(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    grant: &grant::Model,
    previous_state: Option<Value>,
) -> Result<(), ApiError> {
    let now = Utc::now().naive_utc();
    let (authentication_method, session_reference) = match context.credential_kind {
        CredentialKind::ApiSession => (
            "api_session",
            format!("api_session:{}", context.credential_reference),
        ),
        CredentialKind::ApiAppToken => (
            "api_app_token",
            format!("api_app_token:{}", context.credential_reference),
        ),
        CredentialKind::OauthGrant => (
            "oauth",
            format!("oauth_grant:{}", context.credential_reference),
        ),
        CredentialKind::BrowserSession => (
            "browser_session",
            format!("browser_session:{}", context.credential_reference),
        ),
    };
    security_audit_event::ActiveModel {
        household_id: Set(context.membership.household_id),
        actor_account_id: Set(Some(context.account_id)),
        actor_membership_id: Set(Some(context.membership.id)),
        event_type: Set("household_access.person_grant_changed".to_owned()),
        request_id: Set(Some(request_id.to_owned())),
        metadata: Set(json!({
            "target_membership_id": grant.household_membership_id,
            "target_grant_id": grant.id,
            "previous_state": previous_state,
            "new_state": grant_state(grant),
            "outcome": "success"
        })),
        audit_context: Set(json!({
            "actor_account_id": context.account_id,
            "actor_user_id": context.user_id,
            "actor_membership_id": context.membership.id,
            "active_role": context.membership.role,
            "permissions_version": context.membership.permissions_version,
            "household_id": context.membership.household_id,
            "authentication_method": authentication_method,
            "session_reference": session_reference,
            "request_id": request_id
        })),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    Ok(())
}

pub(super) async fn representation(
    db: &DatabaseTransaction,
    record: person::Model,
) -> Result<(Value, String), ApiError> {
    let row = serialize_people(db, vec![record]).await?.remove(0);
    let body = json!({"data": row});
    let etag = representation_etag(&body);
    Ok((body, etag))
}
