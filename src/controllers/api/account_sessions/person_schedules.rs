use super::*;
use crate::models::care::{
    person_schedules::{self, ClinicalRevision, PersonScheduleId, PersonalAction},
    treatments,
};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AdoptionInput {
    source_portable_id: String,
}

pub(super) async fn adopt(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    request: Option<axum::Extension<LocoRequestId>>,
    body: std::result::Result<Json<AdoptionInput>, JsonRejection>,
) -> Response {
    let request_id = request_id(request);
    let Json(input) = match body {
        Ok(input) => input,
        Err(_) => {
            return operation(
                crate::models::profile::validation("source_portable_id", "is required"),
                &request_id,
            );
        }
    };
    let principal =
        match resource::authenticate(&ctx.db, &headers, ctx.config.settings.as_ref()).await {
            Ok(principal) => principal,
            Err(error) => return authentication(error, &request_id),
        };
    let transaction = match principal
        .personal_plan_transaction(&ctx.db, ctx.config.settings.as_ref(), true)
        .await
    {
        Ok(transaction) => transaction,
        Err(error) => return authentication(error, &request_id),
    };
    match person_schedules::legacy::adopt(
        &transaction,
        &principal,
        &input.source_portable_id,
        &request_id,
        ctx.config.settings.as_ref(),
    )
    .await
    {
        Ok(plan) => match transaction.commit().await {
            Ok(()) => (
                StatusCode::CREATED,
                [
                    (header::CACHE_CONTROL, "no-store"),
                    (header::ETAG, plan.etag.as_str()),
                ],
                Json(json!({"data":plan})),
            )
                .into_response(),
            Err(error) => operation(error.into(), &request_id),
        },
        Err(person_schedules::shared_commands::Failure::Operation(error)) => {
            operation(error, &request_id)
        }
        Err(person_schedules::shared_commands::Failure::Authentication(error)) => {
            authentication(error, &request_id)
        }
    }
}

pub(super) async fn index(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    request: Option<axum::Extension<LocoRequestId>>,
) -> Response {
    let request_id = request_id(request);
    let principal =
        match resource::authenticate(&ctx.db, &headers, ctx.config.settings.as_ref()).await {
            Ok(value) => value,
            Err(error) => return authentication(error, &request_id),
        };
    let transaction = match principal
        .personal_plan_transaction(&ctx.db, ctx.config.settings.as_ref(), false)
        .await
    {
        Ok(value) => value,
        Err(error) => return authentication(error, &request_id),
    };
    let result = person_schedules::own_plans(
        &transaction,
        AccountId::from_database(principal.account_id()),
    )
    .await;
    match result {
        Ok(plans) => {
            if let Err(error) = person_schedules::audit_read(
                &transaction,
                AccountId::from_database(principal.account_id()),
                &request_id,
                principal.provenance(),
            )
            .await
            {
                return operation(error, &request_id);
            }
            match transaction.commit().await {
                Ok(()) => (
                    [(header::CACHE_CONTROL, "no-store")],
                    Json(json!({"data":plans})),
                )
                    .into_response(),
                Err(error) => operation(error.into(), &request_id),
            }
        }
        Err(error) => operation(error, &request_id),
    }
}

pub(super) async fn update(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    request: Option<axum::Extension<LocoRequestId>>,
    body: std::result::Result<Json<Value>, JsonRejection>,
) -> Response {
    let request_id = request_id(request);
    let result = async {
        let id = PersonScheduleId::parse(&id)?;
        let etag = headers
            .get(header::IF_MATCH)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| crate::models::profile::validation("if_match", "is required"))?;
        let revision = etag
            .strip_prefix(&format!("\"person-schedule:{}:", id.as_str()))
            .and_then(|value| value.strip_suffix('"'))
            .and_then(|value| value.parse::<i64>().ok())
            .ok_or_else(|| crate::models::profile::validation("if_match", "is invalid"))?;
        let revision = ClinicalRevision::parse(revision)?;
        let Json(body) =
            body.map_err(|_| crate::models::profile::validation("schedule", "is invalid"))?;
        let action = if let Some(action) = body.get("action").and_then(Value::as_str) {
            if body.as_object().is_none_or(|body| body.len() != 1) {
                return Err(crate::models::profile::validation("action", "is invalid"));
            }
            match action {
                "pause" => PersonalAction::Pause,
                "resume" => PersonalAction::Resume,
                "retire" => PersonalAction::Retire,
                _ => return Err(crate::models::profile::validation("action", "is invalid")),
            }
        } else {
            PersonalAction::Edit(treatments::parse_personal_input(&body)?)
        };
        Ok((id, revision, action))
    }
    .await;
    let (id, revision, action) = match result {
        Ok(value) => value,
        Err(error) => return operation(error, &request_id),
    };
    let principal =
        match resource::authenticate(&ctx.db, &headers, ctx.config.settings.as_ref()).await {
            Ok(value) => value,
            Err(error) => return authentication(error, &request_id),
        };
    let transaction = match principal
        .personal_plan_transaction(&ctx.db, ctx.config.settings.as_ref(), true)
        .await
    {
        Ok(value) => value,
        Err(error) => return authentication(error, &request_id),
    };
    let result = person_schedules::update_personal(
        &transaction,
        AccountId::from_database(principal.account_id()),
        id,
        revision,
        action,
        &request_id,
        principal.provenance(),
    )
    .await;
    match result {
        Ok(plan) => match transaction.commit().await {
            Ok(()) => (
                [
                    (header::CACHE_CONTROL, "no-store"),
                    (header::ETAG, plan.etag().as_str()),
                ],
                Json(json!({"data":plan})),
            )
                .into_response(),
            Err(error) => operation(error.into(), &request_id),
        },
        Err(error) => operation(error, &request_id),
    }
}

pub(super) async fn withdraw_sharing(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    request: Option<axum::Extension<LocoRequestId>>,
) -> Response {
    let request_id = request_id(request);
    let principal =
        match resource::authenticate(&ctx.db, &headers, ctx.config.settings.as_ref()).await {
            Ok(value) => value,
            Err(error) => return authentication(error, &request_id),
        };
    let transaction = match principal
        .guardian_declaration_transaction(&ctx.db, ctx.config.settings.as_ref(), true)
        .await
    {
        Ok(value) => value,
        Err(error) => return authentication(error, &request_id),
    };
    match person_schedules::sharing::withdraw(
        &transaction,
        AccountId::from_database(principal.account_id()),
        id,
        &request_id,
        principal.provenance(),
    )
    .await
    {
        Ok(()) => match transaction.commit().await {
            Ok(()) => (
                StatusCode::NO_CONTENT,
                [(header::CACHE_CONTROL, "no-store")],
            )
                .into_response(),
            Err(error) => operation(error.into(), &request_id),
        },
        Err(error) => operation(error, &request_id),
    }
}

fn operation(error: OperationError, request_id: &str) -> Response {
    match error {
        OperationError::Conflict { .. } => failure(
            StatusCode::CONFLICT,
            "conflict",
            "Record has changed since it was last read",
            request_id,
        ),
        OperationError::Validation { details } => reply(
            StatusCode::UNPROCESSABLE_ENTITY,
            Some(
                json!({"error":{"code":"validation_failed","message":"Schedule is invalid","request_id":request_id,"details":details}}),
            ),
            request_id,
        ),
        error => super::operation(error, request_id),
    }
}
