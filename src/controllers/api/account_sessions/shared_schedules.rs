use super::*;
use crate::models::care::{
    person_schedules::{self, ClinicalRevision, PersonScheduleId, PersonalAction, shared_commands},
    treatments,
};

pub(super) async fn update(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    request: Option<axum::Extension<LocoRequestId>>,
    body: std::result::Result<Json<Value>, JsonRejection>,
) -> Response {
    let request_id = request_id(request);
    let parsed = (|| {
        let id = PersonScheduleId::parse(&id)?;
        let etag = headers
            .get(header::IF_MATCH)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| crate::models::profile::validation("if_match", "is required"))?;
        let expected = ClinicalRevision::from_etag(&id, etag)?;
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
            PersonalAction::Edit(Box::new(treatments::parse_personal_input(&body)?))
        };
        Ok::<_, OperationError>((id, expected, action))
    })();
    let (id, expected, action) = match parsed {
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
    let result = person_schedules::shared_commands::update(
        &transaction,
        &principal,
        id,
        expected,
        action,
        &request_id,
        ctx.config.settings.as_ref(),
    )
    .await;
    match result {
        Ok(plan) => match transaction.commit().await {
            Ok(()) => (
                [
                    (header::CACHE_CONTROL, "no-store"),
                    (header::ETAG, plan.etag.as_str()),
                ],
                Json(json!({"data":plan})),
            )
                .into_response(),
            Err(error) => operation(error.into(), &request_id),
        },
        Err(error) => {
            if let Err(error) = transaction.rollback().await {
                return operation(error.into(), &request_id);
            }
            match error {
                shared_commands::Failure::Operation(error) => operation(error, &request_id),
                shared_commands::Failure::Authentication(error) => {
                    authentication(error, &request_id)
                }
            }
        }
    }
}
