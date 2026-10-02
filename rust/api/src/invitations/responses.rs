use super::*;

pub(super) const CONTROLLER: &str = "api/v1/admin/invitations";

pub(super) const POLICY: &str = "HouseholdInvitationPolicy";

pub(super) async fn audit_event(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    event_type: &str,
    target_id: i64,
) -> Result<(), ApiError> {
    let now = Utc::now().naive_utc();
    security_audit_event::ActiveModel {
        household_id: Set(context.membership.household_id),
        actor_account_id: Set(Some(context.account_id)),
        actor_membership_id: Set(Some(context.membership.id)),
        event_type: Set(event_type.to_owned()),
        request_id: Set(Some(request_id.to_owned())),
        metadata: Set(
            json!({"target_type":"HouseholdInvitation","target_id":target_id,"outcome":"success"}),
        ),
        audit_context: Set(json!({})),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    Ok(())
}

pub(super) async fn denied(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    action: &str,
) -> Result<Response, ApiError> {
    error_response(
        db,
        context,
        method,
        CONTROLLER,
        POLICY,
        action,
        StatusCode::FORBIDDEN,
        "forbidden",
        "You are not authorized to perform this action.",
        None,
    )
    .await
}

pub(super) async fn invalid(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    action: &str,
    status: StatusCode,
) -> Result<Response, ApiError> {
    let code = if status == StatusCode::BAD_REQUEST {
        "bad_request"
    } else {
        "validation_failed"
    };
    error_response(
        db,
        context,
        method,
        CONTROLLER,
        POLICY,
        action,
        status,
        code,
        "Invalid invitation",
        None,
    )
    .await
}

pub(super) fn no_store(response: &mut Response) {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
}

pub(super) async fn resend_unavailable(
    state: &AppState,
    headers: &HeaderMap,
    household_id: i64,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(state, headers, household_id).await?;
    let mut response = error_response(
        db,
        &context,
        "POST",
        CONTROLLER,
        POLICY,
        "resend",
        StatusCode::SERVICE_UNAVAILABLE,
        "invitation_delivery_unavailable",
        "Invitation delivery is temporarily unavailable",
        None,
    )
    .await?;
    no_store(&mut response);
    Ok(response)
}

pub(super) fn acceptance_error(
    status: StatusCode,
    code: &'static str,
    message: &'static str,
) -> ApiError {
    ApiError {
        status,
        code,
        message,
        preserve_activity: false,
    }
}

pub(super) fn invitation_unavailable() -> ApiError {
    acceptance_error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invitation_unavailable",
        "Invitation is unavailable",
    )
}
