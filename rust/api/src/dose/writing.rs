use super::*;

pub async fn create(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    body: Result<Json<Value>, JsonRejection>,
) -> Response {
    let request_id = Uuid::new_v4().to_string();
    let db = match state.db.begin().await {
        Ok(db) => db,
        Err(error) => return request_error_response(database_error(error), &request_id),
    };
    let context = match authenticate(&state, &db, &headers, household_id).await {
        Ok(context) => context,
        Err(error) => {
            if error.preserve_activity {
                let _ = db.commit().await;
            }
            return request_error_response(error, &request_id);
        }
    };
    let result = match body {
        Ok(Json(body)) => {
            create_with_failure(&db, &context, household_id, &body, &request_id).await
        }
        Err(_) => Err(error(StatusCode::BAD_REQUEST, "Invalid JSON request body").into()),
    };
    match result {
        Ok((status, take)) => {
            let data = match serialize(&db, std::slice::from_ref(&take)).await {
                Ok(mut data) => data.remove(0),
                Err(error) => {
                    let _ = db.rollback().await;
                    return request_error_response(error, &request_id);
                }
            };
            if let Err(error) =
                audit(&db, &context, &request_id, "create", "POST", status, true).await
            {
                let _ = db.rollback().await;
                return request_error_response(error, &request_id);
            }
            if let Err(error) = db.commit().await {
                return request_error_response(database_error(error), &request_id);
            }
            success_response(
                status,
                json!({"data": data}),
                &request_id,
                Some(take_etag(&take)),
            )
        }
        Err(error) => {
            let status = error.error.status;
            let _ = db.rollback().await;
            if let Ok(audit_db) = state.db.begin().await {
                if let Ok(current_context) =
                    authenticate(&state, &audit_db, &headers, household_id).await
                {
                    let _ = audit(
                        &audit_db,
                        &current_context,
                        &request_id,
                        "create",
                        "POST",
                        status,
                        false,
                    )
                    .await;
                }
                let _ = audit_db.commit().await;
            }
            take_error_response(error, &request_id)
        }
    }
}

pub(crate) async fn create_in_transaction(
    db: &DatabaseTransaction,
    context: &AuthContext,
    household_id: i64,
    body: &Value,
    request_id: &str,
) -> Result<(StatusCode, medication_take::Model), ApiError> {
    create_with_failure(db, context, household_id, body, request_id)
        .await
        .map_err(TakeFailure::into_api_error)
}

pub(crate) async fn create_with_failure(
    db: &DatabaseTransaction,
    context: &AuthContext,
    household_id: i64,
    body: &Value,
    request_id: &str,
) -> Result<(StatusCode, medication_take::Model), TakeFailure> {
    let attributes = body
        .get("medication_take")
        .filter(|value| value.is_object())
        .ok_or_else(|| error(StatusCode::BAD_REQUEST, "medication_take is required"))?;
    if body.as_object().is_none_or(|object| object.len() != 1) {
        return Err(error(StatusCode::UNPROCESSABLE_ENTITY, "unknown request field").into());
    }
    let allowed = [
        "client_uuid",
        "source_type",
        "source_id",
        "taken_at",
        "dose_amount",
        "dose_unit",
        "taken_from_medication_id",
    ];
    if attributes
        .as_object()
        .is_some_and(|object| object.keys().any(|key| !allowed.contains(&key.as_str())))
    {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "unknown medication_take field",
        )
        .into());
    }
    if !attributes
        .get("source_id")
        .and_then(Value::as_str)
        .is_some_and(valid_identifier)
        || !attributes
            .get("source_type")
            .and_then(Value::as_str)
            .is_some_and(|kind| !kind.trim().is_empty())
    {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid medication source",
        )
        .into());
    }
    if !attributes
        .get("source_type")
        .and_then(Value::as_str)
        .is_some_and(|kind| matches!(kind, "schedule" | "person_medication"))
    {
        return Err(ApiError::not_found().into());
    }
    if attributes
        .get("client_uuid")
        .is_some_and(|value| !value.as_str().is_some_and(|id| Uuid::parse_str(id).is_ok()))
    {
        return Err(error(StatusCode::UNPROCESSABLE_ENTITY, "invalid client_uuid").into());
    }
    if attributes
        .get("dose_unit")
        .is_some_and(|value| !value.as_str().is_some_and(|unit| !unit.is_empty()))
    {
        return Err(error(StatusCode::UNPROCESSABLE_ENTITY, "invalid dose_unit").into());
    }
    if attributes
        .get("taken_from_medication_id")
        .is_some_and(|value| !value.as_i64().is_some_and(|id| id > 0))
    {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid taken_from_medication_id",
        )
        .into());
    }
    lock_row(db, "households", household_id).await?;
    let current_membership = crate::entities::membership::Entity::find_by_id(context.membership.id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::forbidden)?;
    if current_membership.status != "active"
        || current_membership.revoked_at.is_some()
        || current_membership.permissions_version != context.membership.permissions_version
        || current_membership.role != context.membership.role
        || current_membership.household_id != household_id
    {
        return Err(ApiError::forbidden().into());
    }
    let account = crate::entities::account::Entity::find_by_id(context.account_id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    if account.status != 2 {
        return Err(ApiError::unauthorized().into());
    }
    let lockout = crate::entities::account_lockout::Entity::find_by_id(context.account_id)
        .one(db)
        .await
        .map_err(database_error)?;
    if lockout.is_some_and(|value| value.deadline > Utc::now().naive_utc()) {
        return Err(ApiError::unauthorized().into());
    }
    let client_uuid = attributes
        .get("client_uuid")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty());
    if let Some(client_uuid) = client_uuid {
        lock_client_uuid(db, client_uuid).await?;
    }
    if let Some(client_uuid) = client_uuid {
        if let Some(existing) = medication_take::Entity::find()
            .filter(medication_take::Column::HouseholdId.eq(household_id))
            .filter(medication_take::Column::ClientUuid.eq(client_uuid))
            .one(db)
            .await
            .map_err(database_error)?
        {
            let replay =
                match replay_matches(db, context, household_id, &existing, attributes).await {
                    Ok(replay) => replay,
                    Err(problem) if problem.status == StatusCode::FORBIDDEN => {
                        prepare(db, context, household_id, attributes).await?;
                        return Err(ApiError {
                            status: StatusCode::CONFLICT,
                            code: "idempotency_key_unavailable",
                            message: "Medication take idempotency key is unavailable",
                            preserve_activity: false,
                        }
                        .into());
                    }
                    Err(problem) => return Err(problem.into()),
                };
            if !replay {
                return Err(error(
                    StatusCode::CONFLICT,
                    "client_uuid was already used for a different dose",
                )
                .into());
            }
            return Ok((StatusCode::OK, existing));
        }
    }
    let proposed = prepare(db, context, household_id, attributes).await?;
    if attributes
        .get("dose_unit")
        .is_some_and(|unit| !unit.is_null() && unit.as_str() != Some(proposed.unit.as_str()))
    {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "dose_unit does not match the source",
        )
        .into());
    }
    if !timing_allowed(db, &proposed).await? {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Cannot take medication: timing restrictions not met",
        )
        .into());
    }
    let take = insert_take(db, household_id, &proposed, client_uuid)
        .await?
        .ok_or_else(|| {
            error(
                StatusCode::CONFLICT,
                "client_uuid was already used for a different dose",
            )
        })?;
    decrement_stock(db, context, request_id, &proposed).await?;
    record_domain_audit(db, context, request_id, &take, &proposed).await?;
    Ok((StatusCode::CREATED, take))
}
