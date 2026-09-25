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
) -> Result<(), DbErr> {
    let request_id = Uuid::new_v4().to_string();
    let (authentication_method, session_reference) = match context.credential_kind {
        CredentialKind::ApiSession => (
            "api_session",
            format!("api_session:{}", context.credential_id),
        ),
        CredentialKind::OauthGrant => ("oauth", format!("oauth_grant:{}", context.credential_id)),
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
        audit_context["policy_class"] = json!("MedicationPolicy");
        audit_context["policy_query"] = json!(format!("{action}?"));
    }
    let now = Utc::now().naive_utc();
    security_audit_event::ActiveModel {
        household_id: Set(context.membership.household_id),
        actor_account_id: Set(Some(context.account_id)),
        actor_membership_id: Set(Some(context.membership.id)),
        event_type: Set("api.request".to_owned()),
        request_id: Set(Some(request_id)),
        metadata: Set(json!({
            "http_method": "GET",
            "controller": "api/v1/medications",
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
