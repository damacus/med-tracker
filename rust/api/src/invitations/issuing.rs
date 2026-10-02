use super::*;

pub(crate) async fn create(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    if !household_manager(&context) {
        return denied(db, &context, "POST", "create").await;
    }
    let Json(body) = match payload {
        Ok(body) => body,
        Err(_) => return invalid(db, &context, "POST", "create", StatusCode::BAD_REQUEST).await,
    };
    let path = collection_path(household_id);
    if let Some(response) =
        keyed_replay(&db, &context, &headers, "POST", &path, &body, "create").await?
    {
        db.commit().await.map_err(database_error)?;
        return Ok(response);
    }
    let (email, role) = match parse_create(&body) {
        Ok(value) => value,
        Err(status) => {
            let invalid_email_value = body
                .get("household_invitation")
                .and_then(Value::as_object)
                .and_then(|attributes| attributes.get("email"))
                .and_then(Value::as_str)
                .map(|value| value.trim().to_lowercase())
                .is_some_and(|value| !valid_email(&value));
            if status == StatusCode::UNPROCESSABLE_ENTITY && invalid_email_value {
                return keyed_failure(
                    db,
                    &context,
                    &headers,
                    "POST",
                    &path,
                    &body,
                    "create",
                    status,
                    "validation_failed",
                    "Validation failed",
                    Some(json!({"email": ["is invalid"]})),
                )
                .await;
            }
            let code = if status == StatusCode::BAD_REQUEST {
                "bad_request"
            } else {
                "validation_failed"
            };
            return keyed_failure(
                db,
                &context,
                &headers,
                "POST",
                &path,
                &body,
                "create",
                status,
                code,
                "Invalid invitation",
                None,
            )
            .await;
        }
    };
    let duplicate = household_invitation::Entity::find()
        .filter(household_invitation::Column::HouseholdId.eq(household_id))
        .filter(household_invitation::Column::Email.eq(&email))
        .filter(household_invitation::Column::AcceptedAt.is_null())
        .filter(household_invitation::Column::RevokedAt.is_null())
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
            &body,
            "create",
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Validation failed",
            Some(json!({"email": ["has already been taken"]})),
        )
        .await;
    }
    let (_, digest) = new_token()?;
    let now = Utc::now().naive_utc();
    let row = household_invitation::ActiveModel {
        household_id: Set(household_id),
        invited_by_membership_id: Set(context.membership.id),
        email: Set(email),
        membership_role: Set(role),
        token_digest: Set(digest),
        expires_at: Set(now + Duration::days(7)),
        accepted_at: Set(None),
        revoked_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .map_err(database_error)?;
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "HouseholdInvitation",
        row.id,
        "create",
        None,
        Some(state_row(&row)),
    )
    .await?;
    audit_event(
        &db,
        &context,
        &request_id,
        "api/admin/invitation/created",
        row.id,
    )
    .await?;
    let response_body = json!({"data":summary(&row, now)});
    if let Some(key) = mutation_idempotency::key(&headers) {
        mutation_idempotency::store(
            &db,
            &context,
            StoredResponse {
                key,
                method: "POST",
                path: &path,
                digest: &mutation_idempotency::digest("POST", &path, &body),
                status: StatusCode::CREATED,
                body: response_body.clone(),
                request_id: &request_id,
                etag: None,
            },
        )
        .await?;
    }
    finish_with_request_id(
        db,
        &context,
        &request_id,
        "POST",
        CONTROLLER,
        POLICY,
        "create",
        StatusCode::CREATED,
        true,
        response_body,
        None,
    )
    .await
}

pub(crate) async fn destroy(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    if !household_manager(&context) {
        return denied(db, &context, "DELETE", "destroy").await;
    }
    let path = format!("/api/v1/households/{household_id}/admin/invitations/{id}");
    let request = json!({});
    if let Some(response) = keyed_replay(
        &db, &context, &headers, "DELETE", &path, &request, "destroy",
    )
    .await?
    {
        db.commit().await.map_err(database_error)?;
        return Ok(response);
    }
    let Some(id) = valid_id(&id) else {
        return keyed_failure(
            db,
            &context,
            &headers,
            "DELETE",
            &path,
            &request,
            "destroy",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
            None,
        )
        .await;
    };
    let row = household_invitation::Entity::find_by_id(id)
        .filter(household_invitation::Column::HouseholdId.eq(household_id))
        .one(&db)
        .await
        .map_err(database_error)?;
    let Some(row) = row else {
        return keyed_failure(
            db,
            &context,
            &headers,
            "DELETE",
            &path,
            &request,
            "destroy",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
            None,
        )
        .await;
    };
    let before = state_row(&row);
    let mut active: household_invitation::ActiveModel = row.into();
    let now = Utc::now().naive_utc();
    active.revoked_at = Set(Some(now));
    active.updated_at = Set(now);
    let row = active.update(&db).await.map_err(database_error)?;
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "HouseholdInvitation",
        row.id,
        "update",
        Some(before),
        Some(state_row(&row)),
    )
    .await?;
    audit_event(
        &db,
        &context,
        &request_id,
        "api/admin/invitation/revoked",
        row.id,
    )
    .await?;
    if let Some(key) = mutation_idempotency::key(&headers) {
        mutation_idempotency::store(
            &db,
            &context,
            StoredResponse {
                key,
                method: "DELETE",
                path: &path,
                digest: &mutation_idempotency::digest("DELETE", &path, &request),
                status: StatusCode::NO_CONTENT,
                body: json!({}),
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
        "DELETE",
        CONTROLLER,
        POLICY,
        "destroy",
        StatusCode::NO_CONTENT,
        true,
        json!({}),
        None,
    )
    .await?;
    *response.body_mut() = Body::empty();
    response.headers_mut().remove(header::CONTENT_TYPE);
    Ok(response)
}
