use crate::entities::security_audit_event;
use crate::{AuthContext, CredentialKind};
use axum::http::StatusCode;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseTransaction, DbErr, Set};
use serde_json::json;
use uuid::Uuid;

pub async fn record_medication_read(
    db: &DatabaseTransaction,
    context: &AuthContext,
    action: &str,
    status: StatusCode,
    authorized: bool,
) -> Result<String, DbErr> {
    record_resource_read(
        db,
        context,
        "api/v1/medications",
        "MedicationPolicy",
        action,
        status,
        authorized,
    )
    .await
}

pub async fn record_resource_read(
    db: &DatabaseTransaction,
    context: &AuthContext,
    controller: &str,
    policy_class: &str,
    action: &str,
    status: StatusCode,
    authorized: bool,
) -> Result<String, DbErr> {
    record_resource_request(
        db,
        context,
        "GET",
        controller,
        policy_class,
        action,
        status,
        authorized,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn record_resource_request(
    db: &DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    controller: &str,
    policy_class: &str,
    action: &str,
    status: StatusCode,
    authorized: bool,
) -> Result<String, DbErr> {
    let request_id = Uuid::new_v4().to_string();
    record_resource_request_with_id(
        db,
        context,
        &request_id,
        method,
        controller,
        policy_class,
        action,
        status,
        authorized,
    )
    .await?;
    Ok(request_id)
}

#[allow(clippy::too_many_arguments)]
pub async fn record_resource_request_with_id(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    method: &str,
    controller: &str,
    policy_class: &str,
    action: &str,
    status: StatusCode,
    authorized: bool,
) -> Result<(), DbErr> {
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
    let mut audit_context = json!({
        "actor_account_id": context.account_id,
        "actor_user_id": context.user_id,
        "actor_membership_id": context.membership.id,
        "active_role": context.membership.role,
        "permissions_version": context.membership.permissions_version,
        "household_id": context.membership.household_id,
        "authentication_method": authentication_method,
        "session_reference": session_reference,
        "request_id": request_id
    });
    if authorized {
        audit_context["policy_class"] = json!(policy_class);
        audit_context["policy_query"] = json!(format!("{action}?"));
    }
    let now = Utc::now().naive_utc();
    security_audit_event::ActiveModel {
        household_id: Set(context.membership.household_id),
        actor_account_id: Set(Some(context.account_id)),
        actor_membership_id: Set(Some(context.membership.id)),
        event_type: Set("api.request".to_owned()),
        request_id: Set(Some(request_id.to_owned())),
        metadata: Set(json!({
            "http_method": method,
            "controller": controller,
            "action": action,
            "outcome": if status.as_u16() < 400 { "success" } else { "failure" },
            "status": status.as_u16()
        })),
        audit_context: Set(audit_context),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}

pub async fn record_cookie_write_denial(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    method: &str,
    status: StatusCode,
) -> Result<(), DbErr> {
    let now = Utc::now().naive_utc();
    security_audit_event::ActiveModel {
        household_id: Set(context.membership.household_id),
        actor_account_id: Set(Some(context.account_id)),
        actor_membership_id: Set(Some(context.membership.id)),
        event_type: Set("api.request".to_owned()),
        request_id: Set(Some(request_id.to_owned())),
        metadata: Set(json!({
            "http_method": method,
            "controller": "api/v1/medication_takes",
            "action": "create",
            "outcome": "failure",
            "status": status.as_u16()
        })),
        audit_context: Set(json!({
            "actor_account_id": context.account_id,
            "actor_user_id": context.user_id,
            "actor_membership_id": context.membership.id,
            "active_role": context.membership.role,
            "permissions_version": context.membership.permissions_version,
            "household_id": context.membership.household_id,
            "authentication_method": "browser_session",
            "session_reference": format!("browser_session:{}", context.credential_reference),
            "request_id": request_id
        })),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}
