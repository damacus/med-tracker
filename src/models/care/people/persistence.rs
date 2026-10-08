use super::*;

pub(super) fn snapshot(record: &person::Model) -> Value {
    json!({"id":record.id,"account_id":record.account_id,"household_id":record.household_id,"portable_id":record.portable_id,"name":record.name,"email":record.email,"date_of_birth":record.date_of_birth,"person_type":record.person_type,"has_capacity":record.has_capacity,"created_at":record.created_at,"updated_at":record.updated_at})
}
pub(super) fn grant_state(record: &grant::Model) -> Value {
    json!({"household_membership_id":record.household_membership_id,"person_id":record.person_id,"access_level":record.access_level,"relationship_type":record.relationship_type,"expires_at":record.expires_at,"revoked_at":record.revoked_at,"carer_relationship_id":record.carer_relationship_id})
}

fn context(
    tenant: &TenantTransaction,
    event: &str,
    provenance: Option<&CredentialProvenance>,
) -> Value {
    let mut value = json!({"actor_account_id":tenant.scope().actor.account_id,"actor_user_id":tenant.user_id(),"actor_membership_id":tenant.membership().id,"active_role":tenant.membership().role,"permissions_version":tenant.membership().permissions_version,"household_id":tenant.scope().household_id,"request_id":tenant.scope().request_id,"policy_class":"PersonPolicy","policy_query":format!("{event}?")});
    if let Some(provenance) = provenance {
        let (method, prefix) = match provenance.method {
            CredentialMethod::ApiSession => ("api_session", "api_session"),
            CredentialMethod::ApiAppToken => ("api_app_token", "api_app_token"),
            CredentialMethod::OauthGrant => ("oauth", "oauth_grant"),
            CredentialMethod::PersonalApiKey => ("personal_api_key", "personal_api_key"),
            CredentialMethod::BrowserSession => ("browser_session", "browser_session"),
        };
        value["authentication_method"] = json!(method);
        value["session_reference"] = json!(format!("{prefix}:{}", provenance.reference));
    }
    value
}

pub(super) async fn record_version(
    tenant: &TenantTransaction,
    kind: &str,
    id: i64,
    event: &str,
    before: Option<Value>,
    after: Value,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    let mut fields = std::collections::BTreeSet::new();
    for row in [before.as_ref(), Some(&after)].into_iter().flatten() {
        fields.extend(
            row.as_object()
                .ok_or(OperationError::Unavailable)?
                .keys()
                .cloned(),
        );
    }
    let mut changes = serde_json::Map::new();
    for field in fields {
        let old = before
            .as_ref()
            .and_then(|value| value.get(&field))
            .cloned()
            .unwrap_or(Value::Null);
        let new = after.get(&field).cloned().unwrap_or(Value::Null);
        if old != new {
            changes.insert(field, json!([old, new]));
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
        audit_context: Set(context(tenant, event, provenance)),
        created_at: Set(Some(Utc::now().naive_utc())),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await?;
    Ok(())
}

pub(super) async fn change(
    tenant: &TenantTransaction,
    kind: &str,
    id: i64,
    portable: &str,
    event: &str,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    let now = Utc::now().naive_utc();
    let _ = provenance;
    api_change_event::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        household_membership_id: Set(Some(tenant.membership().id)),
        account_id: Set(Some(tenant.scope().actor.account_id)),
        action: Set(event.into()),
        record_type: Set(kind.into()),
        record_id: Set(id),
        record_portable_id: Set(Some(portable.into())),
        request_id: Set(Some(tenant.scope().request_id.clone())),
        metadata: Set(json!({"record_type":kind,"record_id":id,"portable_id":portable})),
        occurred_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await?;
    Ok(())
}

pub(super) async fn grant_event(
    tenant: &TenantTransaction,
    record: &grant::Model,
    before: Option<Value>,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    let now = Utc::now().naive_utc();
    security_audit_event::ActiveModel{household_id:Set(tenant.scope().household_id),actor_account_id:Set(Some(tenant.scope().actor.account_id)),actor_membership_id:Set(Some(tenant.membership().id)),event_type:Set("household_access.person_grant_changed".into()),request_id:Set(Some(tenant.scope().request_id.clone())),metadata:Set(json!({"target_membership_id":record.household_membership_id,"target_grant_id":record.id,"previous_state":before,"new_state":grant_state(record),"outcome":"success"})),audit_context:Set(context(tenant,"create",provenance)),created_at:Set(now),updated_at:Set(now),..Default::default()}.insert(tenant.transaction()).await?;
    Ok(())
}
