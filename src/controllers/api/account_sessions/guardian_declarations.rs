use super::*;
use crate::models::care::guardian_declarations;
use axum::extract::{Query, rejection::QueryRejection};

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Pagination {
    limit: Option<u16>,
    before_id: Option<i64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WithdrawalBody {
    guardian_declaration: Withdrawal,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Withdrawal {
    withdrawn: bool,
    revision: String,
}

pub(super) async fn index(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    request: Option<axum::Extension<LocoRequestId>>,
    query: std::result::Result<Query<Pagination>, QueryRejection>,
) -> Response {
    let request_id = request_id(request);
    let Ok(Query(query)) = query else {
        return invalid(&request_id);
    };
    let limit = query.limit.unwrap_or(25);
    if !(1..=100).contains(&limit) || query.before_id.is_some_and(|id| id <= 0) {
        return invalid(&request_id);
    }
    let principal =
        match resource::authenticate(&ctx.db, &headers, ctx.config.settings.as_ref()).await {
            Ok(principal) => principal,
            Err(error) => return authentication(error, &request_id),
        };
    let transaction = match principal
        .guardian_declaration_transaction(&ctx.db, ctx.config.settings.as_ref(), false)
        .await
    {
        Ok(transaction) => transaction,
        Err(error) => return authentication(error, &request_id),
    };
    let result = guardian_declarations::own_page(
        &transaction,
        principal.account_id(),
        limit,
        query.before_id,
    )
    .await
    .map(|page| json!({"data":page.records,"meta":{"next_before_id":page.next_before_id}}));
    finish(transaction, result, &request_id).await
}

pub(super) async fn withdraw(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    request: Option<axum::Extension<LocoRequestId>>,
    id: std::result::Result<Path<String>, PathRejection>,
    body: std::result::Result<Json<WithdrawalBody>, JsonRejection>,
) -> Response {
    let request_id = request_id(request);
    let Some(id) = id
        .ok()
        .and_then(|Path(id)| id.parse::<i64>().ok())
        .filter(|id| *id > 0)
    else {
        return invalid(&request_id);
    };
    let Ok(Json(body)) = body else {
        return invalid(&request_id);
    };
    let Some(revision) = body
        .guardian_declaration
        .revision
        .parse::<i64>()
        .ok()
        .filter(|id| *id >= 0)
    else {
        return invalid(&request_id);
    };
    if !body.guardian_declaration.withdrawn {
        return invalid(&request_id);
    }
    let principal =
        match resource::authenticate(&ctx.db, &headers, ctx.config.settings.as_ref()).await {
            Ok(principal) => principal,
            Err(error) => return authentication(error, &request_id),
        };
    let transaction = match principal
        .guardian_declaration_transaction(&ctx.db, ctx.config.settings.as_ref(), true)
        .await
    {
        Ok(transaction) => transaction,
        Err(error) => return authentication(error, &request_id),
    };
    let result = guardian_declarations::withdraw_own(
        &transaction,
        principal.account_id(),
        id,
        revision,
        &request_id,
        principal.provenance(),
    )
    .await
    .map(|record| json!({"data":record}));
    finish(transaction, result, &request_id).await
}

fn invalid(request_id: &str) -> Response {
    failure(
        StatusCode::UNPROCESSABLE_ENTITY,
        "unprocessable_content",
        "Invalid guardian declaration request",
        request_id,
    )
}

async fn finish(
    transaction: sea_orm::DatabaseTransaction,
    result: std::result::Result<Value, OperationError>,
    request_id: &str,
) -> Response {
    match result {
        Ok(value) => match transaction.commit().await {
            Ok(()) => reply(StatusCode::OK, Some(value), request_id),
            Err(error) => operation(error.into(), request_id),
        },
        Err(error) => {
            if let Err(error) = transaction.rollback().await {
                return operation(error.into(), request_id);
            }
            match error {
                OperationError::Conflict { code, .. } => failure(
                    StatusCode::CONFLICT,
                    &code,
                    "Guardian declaration revision has changed",
                    request_id,
                ),
                OperationError::Validation { .. } => invalid(request_id),
                other => operation(other, request_id),
            }
        }
    }
}
