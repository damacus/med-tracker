use super::*;

pub(super) async fn keyed_replay(
    db: &DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    method: &str,
    path: &str,
    body: &Value,
    action: &str,
) -> Result<Option<Response>, ApiError> {
    let Some(key) = mutation_idempotency::key(headers) else {
        return Ok(None);
    };
    let digest = mutation_idempotency::digest(method, path, body);
    match mutation_idempotency::lookup(db, context, key, method, path, &digest).await? {
        Lookup::New => Ok(None),
        Lookup::Replay(saved) => {
            let mut saved = *saved;
            let status = StatusCode::from_u16(saved.response_status as u16)
                .map_err(|_| ApiError::internal())?;
            let request_id = audit::record_resource_request(
                db, context, method, CONTROLLER, POLICY, action, status, true,
            )
            .await
            .map_err(database_error)?;
            if saved.response_body.get("error").is_some() {
                saved.response_body["error"]["request_id"] = json!(request_id);
            }
            let mut response = mutation_idempotency::replay(saved)?;
            response.headers_mut().insert(
                "x-request-id",
                HeaderValue::from_str(&request_id).map_err(|_| ApiError::internal())?,
            );
            Ok(Some(response))
        }
        Lookup::Conflict => {
            let request_id = audit::record_resource_request(
                db,
                context,
                method,
                CONTROLLER,
                POLICY,
                action,
                StatusCode::CONFLICT,
                true,
            )
            .await
            .map_err(database_error)?;
            let mut response = (StatusCode::CONFLICT, Json(json!({"error":{"code":"idempotency_key_reused","message":"Idempotency key has already been used for a different request","request_id":request_id}}))).into_response();
            response.headers_mut().insert(
                "x-request-id",
                HeaderValue::from_str(&request_id).map_err(|_| ApiError::internal())?,
            );
            Ok(Some(response))
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn keyed_failure(
    db: DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    method: &str,
    path: &str,
    request: &Value,
    action: &str,
    status: StatusCode,
    code: &str,
    message: &str,
    errors: Option<Value>,
) -> Result<Response, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let mut response_body =
        json!({"error":{"code":code,"message":message,"request_id":request_id}});
    if let Some(errors) = errors {
        response_body["error"]["errors"] = errors;
    }
    if let Some(key) = mutation_idempotency::key(headers) {
        mutation_idempotency::store(
            &db,
            context,
            StoredResponse {
                key,
                method,
                path,
                digest: &mutation_idempotency::digest(method, path, request),
                status,
                body: response_body.clone(),
                request_id: &request_id,
                etag: None,
            },
        )
        .await?;
    }
    finish_with_request_id(
        db,
        context,
        &request_id,
        method,
        CONTROLLER,
        POLICY,
        action,
        status,
        true,
        response_body,
        None,
    )
    .await
}

pub(super) async fn accepted_retry(
    db: &DatabaseTransaction,
    actor: &auth_sessions::InvitationActor,
    headers: &HeaderMap,
    digest: &str,
) -> Result<Option<membership::Model>, ApiError> {
    let memberships = membership::Entity::find()
        .filter(membership::Column::AccountId.eq(actor.account.id))
        .filter(membership::Column::Status.eq("active"))
        .filter(membership::Column::RevokedAt.is_null())
        .order_by_asc(membership::Column::HouseholdId)
        .order_by_asc(membership::Column::Id)
        .all(db)
        .await
        .map_err(database_error)?;
    for row in memberships {
        tenant_setting(db, "med_tracker.current_household_id", row.household_id)
            .await
            .map_err(database_error)?;
        let home = household::Entity::find_by_id(row.household_id)
            .lock_exclusive()
            .one(db)
            .await
            .map_err(database_error)?;
        if !home.is_some_and(|home| home.status == "active" && home.lifecycle_state == "active") {
            continue;
        }
        let current_actor = auth_sessions::invitation_actor(db, headers).await?;
        if current_actor.account.id != actor.account.id
            || current_actor.session.id != actor.session.id
        {
            return Err(invitation_unavailable());
        }
        let current_membership = membership::Entity::find_by_id(row.id)
            .lock_exclusive()
            .one(db)
            .await
            .map_err(database_error)?;
        let Some(current_membership) = current_membership.filter(|member| {
            member.account_id == actor.account.id
                && member.status == "active"
                && member.revoked_at.is_none()
        }) else {
            continue;
        };
        let accepted = household_invitation::Entity::find()
            .filter(household_invitation::Column::HouseholdId.eq(row.household_id))
            .filter(household_invitation::Column::TokenDigest.eq(digest))
            .filter(household_invitation::Column::Email.eq(&actor.account.email))
            .filter(household_invitation::Column::AcceptedAt.is_not_null())
            .filter(household_invitation::Column::RevokedAt.is_null())
            .one(db)
            .await
            .map_err(database_error)?;
        if accepted.is_some() {
            return Ok(Some(current_membership));
        }
    }
    Ok(None)
}
