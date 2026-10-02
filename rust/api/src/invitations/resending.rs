use super::*;

pub(crate) async fn resend(
    state: State<AppState>,
    path: Path<(i64, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    let result = resend_inner(state, path, headers, body).await;
    let mut response = match result {
        Ok(response) => response,
        Err(error) => error.into_response(),
    };
    no_store(&mut response);
    Ok(response)
}

async fn resend_inner(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    if !household_manager(&context)
        || !matches!(
            context.credential_kind,
            CredentialKind::ApiSession | CredentialKind::OauthGrant
        )
    {
        return denied(db, &context, "POST", "resend").await;
    }
    let path = format!("/api/v1/households/{household_id}/admin/invitations/{id}/resend");
    let request = if body.is_empty() {
        json!({})
    } else {
        serde_json::from_slice::<Value>(&body).unwrap_or(Value::Null)
    };
    if let Some(response) =
        keyed_replay(&db, &context, &headers, "POST", &path, &request, "resend").await?
    {
        db.commit().await.map_err(database_error)?;
        return Ok(response);
    }
    let Some(id) = valid_id(&id) else {
        return keyed_failure(
            db,
            &context,
            &headers,
            "POST",
            &path,
            &request,
            "resend",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
            None,
        )
        .await;
    };
    let Some(row) = household_invitation::Entity::find_by_id(id)
        .filter(household_invitation::Column::HouseholdId.eq(household_id))
        .lock_exclusive()
        .one(&db)
        .await
        .map_err(database_error)?
    else {
        return keyed_failure(
            db,
            &context,
            &headers,
            "POST",
            &path,
            &request,
            "resend",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
            None,
        )
        .await;
    };
    if request != json!({}) {
        return keyed_failure(
            db,
            &context,
            &headers,
            "POST",
            &path,
            &request,
            "resend",
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Invalid invitation",
            None,
        )
        .await;
    }
    if row.accepted_at.is_some() || row.revoked_at.is_some() {
        return keyed_failure(
            db,
            &context,
            &headers,
            "POST",
            &path,
            &request,
            "resend",
            StatusCode::UNPROCESSABLE_ENTITY,
            "unprocessable_content",
            "Invitation cannot be resent",
            None,
        )
        .await;
    }
    let key = mutation_idempotency::key(&headers);
    let digest = mutation_idempotency::digest("POST", &path, &request);
    let duplicate = household_invitation::Entity::find()
        .filter(household_invitation::Column::HouseholdId.eq(household_id))
        .filter(household_invitation::Column::Email.eq(&row.email))
        .filter(household_invitation::Column::Id.ne(row.id))
        .filter(household_invitation::Column::AcceptedAt.is_null())
        .filter(household_invitation::Column::RevokedAt.is_null())
        .filter(household_invitation::Column::ExpiresAt.gt(Utc::now().naive_utc()))
        .one(&db)
        .await
        .map_err(database_error)?;
    if duplicate.is_some() {
        return keyed_failure(
            db,
            &context,
            &headers,
            "POST",
            &path,
            &request,
            "resend",
            StatusCode::UNPROCESSABLE_ENTITY,
            "unprocessable_content",
            "Invitation cannot be resent",
            None,
        )
        .await;
    }
    let before = state_row(&row);
    let (token, token_digest) = new_token()?;
    let now = Utc::now().naive_utc();
    let mut active: household_invitation::ActiveModel = row.into();
    active.token_digest = Set(token_digest);
    active.expires_at = Set(now + Duration::days(7));
    active.updated_at = Set(now);
    let row = active.update(&db).await.map_err(database_error)?;
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "HouseholdInvitation",
        row.id,
        "resend",
        Some(before),
        Some(state_row(&row)),
    )
    .await?;
    audit_event(
        &db,
        &context,
        &request_id,
        "api/admin/invitation/resent",
        row.id,
    )
    .await?;
    let config = (*state.invitation_mail).clone();
    let email = row.email.clone();
    let delivered = tokio::task::spawn_blocking(move || smtp_send(config, email, token)).await;
    if !matches!(delivered, Ok(Ok(()))) {
        db.rollback().await.map_err(database_error)?;
        return resend_unavailable(&state, &headers, household_id).await;
    }
    let body = json!({"data":{"invitation_id":row.id.to_string(),"expires_at":row.expires_at.and_utc().to_rfc3339(),"delivery_status":"queued"}});
    if let Some(key) = key {
        mutation_idempotency::store(
            &db,
            &context,
            StoredResponse {
                key,
                method: "POST",
                path: &path,
                digest: &digest,
                status: StatusCode::OK,
                body: body.clone(),
                request_id: &request_id,
                etag: None,
            },
        )
        .await?;
    }
    let mut response = finish_with_request_id(
        db,
        &context,
        &request_id,
        "POST",
        CONTROLLER,
        POLICY,
        "resend",
        StatusCode::OK,
        true,
        body,
        None,
    )
    .await?;
    no_store(&mut response);
    Ok(response)
}
