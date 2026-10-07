use super::*;

fn context(tenant: &TenantTransaction, provenance: Option<&CredentialProvenance>) -> Value {
    let mut context = json!({"actor_account_id":tenant.scope().actor.account_id,"actor_user_id":tenant.user_id(),"actor_membership_id":tenant.membership().id,"active_role":tenant.membership().role,"permissions_version":tenant.membership().permissions_version,"household_id":tenant.scope().household_id,"request_id":tenant.scope().request_id});
    if let Some(provenance) = provenance {
        let (method, prefix) = match provenance.method {
            CredentialMethod::ApiSession => ("api_session", "api_session"),
            CredentialMethod::ApiAppToken => ("api_app_token", "api_app_token"),
            CredentialMethod::OauthGrant => ("oauth", "oauth_grant"),
            CredentialMethod::PersonalApiKey => ("personal_api_key", "personal_api_key"),
            CredentialMethod::BrowserSession => ("browser_session", "browser_session"),
        };
        context["authentication_method"] = json!(method);
        context["session_reference"] = json!(format!("{prefix}:{}", provenance.reference));
    }
    context
}
pub(crate) async fn event(
    tenant: &TenantTransaction,
    event: &str,
    metadata: Value,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    let now = Utc::now().naive_utc();
    security_audit_event::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        actor_account_id: Set(Some(tenant.scope().actor.account_id)),
        actor_membership_id: Set(Some(tenant.membership().id)),
        event_type: Set(event.into()),
        request_id: Set(Some(tenant.scope().request_id.clone())),
        metadata: Set(metadata),
        audit_context: Set(context(tenant, provenance)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await?;
    Ok(())
}
pub(super) async fn record_version(
    tenant: &TenantTransaction,
    kind: &str,
    id: i64,
    before: Option<Value>,
    after: Value,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    record_version_as(
        tenant,
        kind,
        id,
        if before.is_some() { "update" } else { "create" },
        before,
        after,
        provenance,
    )
    .await
}
pub(crate) async fn record_version_as(
    tenant: &TenantTransaction,
    kind: &str,
    id: i64,
    event: &str,
    before: Option<Value>,
    after: Value,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    let mut keys = std::collections::BTreeSet::new();
    for value in [before.as_ref(), Some(&after)].into_iter().flatten() {
        keys.extend(
            value
                .as_object()
                .ok_or(OperationError::Unavailable)?
                .keys()
                .cloned(),
        );
    }
    let mut changes = serde_json::Map::new();
    for key in keys {
        let old = before
            .as_ref()
            .and_then(|value| value.get(&key))
            .cloned()
            .unwrap_or(Value::Null);
        let new = after.get(&key).cloned().unwrap_or(Value::Null);
        if old != new {
            changes.insert(key, json!([old, new]));
        }
    }
    version::ActiveModel {
        item_type: Set(kind.into()),
        item_id: Set(id),
        event: Set(event.into()),
        object: Set(before.map(|value| value.to_string())),
        object_changes: Set(Some(Value::Object(changes).to_string())),
        whodunnit: Set(Some(tenant.user_id().to_string())),
        request_id: Set(Some(tenant.scope().request_id.clone())),
        household_id: Set(Some(tenant.scope().household_id)),
        actor_membership_id: Set(Some(tenant.membership().id)),
        audit_context: Set(context(tenant, provenance)),
        created_at: Set(Some(Utc::now().naive_utc())),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await?;
    Ok(())
}
pub(crate) async fn bump(tenant: &TenantTransaction, id: i64) -> Result<(), OperationError> {
    use sea_orm::sea_query::{Expr, ExprTrait};
    membership::Entity::update_many()
        .col_expr(
            membership::Column::PermissionsVersion,
            Expr::col(membership::Column::PermissionsVersion).add(1),
        )
        .col_expr(
            membership::Column::UpdatedAt,
            Expr::value(Utc::now().naive_utc()),
        )
        .filter(membership::Column::Id.eq(id))
        .filter(membership::Column::HouseholdId.eq(tenant.scope().household_id))
        .exec(tenant.transaction())
        .await?;
    Ok(())
}
