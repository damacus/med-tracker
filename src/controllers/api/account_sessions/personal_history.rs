use super::*;
use crate::models::{
    identity::resource::{HistoryActor, deny_personal_history},
    profile::personal_history,
};
use axum::extract::{Query, rejection::QueryRejection};
use serde::Serialize;

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Pagination {
    before_id: Option<i64>,
    before_taken_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Serialize)]
struct TakenDose {
    id: i64,
    taken_at: chrono::DateTime<chrono::Utc>,
    dose_amount: Option<String>,
    dose_unit: Option<String>,
    medicine: Option<String>,
}

#[derive(Serialize)]
struct Cursor {
    id: i64,
    taken_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
struct Meta {
    next_cursor: Option<Cursor>,
}

#[derive(Serialize)]
struct HistoryResponse {
    data: Vec<TakenDose>,
    meta: Meta,
}

pub(super) async fn index(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    request: Option<axum::Extension<LocoRequestId>>,
    query: Result<Query<Pagination>, QueryRejection>,
) -> Response {
    let request_id = request_id(request);
    let principal =
        match resource::authenticate(&ctx.db, &headers, ctx.config.settings.as_ref()).await {
            Ok(value) => value,
            Err(error) => {
                return refused(&ctx, None, &request_id, authentication(error, &request_id)).await;
            }
        };
    let actor = HistoryActor::resource(&principal);
    let transaction = match principal
        .personal_history_transaction(&ctx.db, ctx.config.settings.as_ref())
        .await
    {
        Ok(value) => value,
        Err(error) => {
            return refused(
                &ctx,
                Some(&actor),
                &request_id,
                authentication(error, &request_id),
            )
            .await;
        }
    };
    let before = match query.map(|Query(page)| (page.before_id, page.before_taken_at)) {
        Ok((None, None)) => None,
        Ok((Some(id), Some(taken_at))) if id > 0 => Some(personal_history::Cursor {
            id,
            taken_at: taken_at.naive_utc(),
        }),
        _ => {
            if let Err(error) = transaction.rollback().await {
                return operation(error.into(), &request_id);
            }
            return refused(
                &ctx,
                Some(&actor),
                &request_id,
                failure(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "unprocessable_content",
                    "Invalid history pagination",
                    &request_id,
                ),
            )
            .await;
        }
    };
    let page = match personal_history::page(&transaction, &actor, before, &request_id).await {
        Ok(value) => value,
        Err(error) => {
            if let Err(error) = transaction.rollback().await {
                return operation(error.into(), &request_id);
            }
            return refused(
                &ctx,
                Some(&actor),
                &request_id,
                operation(error, &request_id),
            )
            .await;
        }
    };
    if let Err(error) = transaction.commit().await {
        return refused(
            &ctx,
            Some(&actor),
            &request_id,
            operation(error.into(), &request_id),
        )
        .await;
    }
    let body = HistoryResponse {
        data: page
            .records
            .into_iter()
            .map(|record| TakenDose {
                id: record.id,
                taken_at: record.taken_at.and_utc(),
                dose_amount: record.dose_amount.map(|amount| amount.to_string()),
                dose_unit: record.dose_unit,
                medicine: record.medicine,
            })
            .collect(),
        meta: Meta {
            next_cursor: page.next.map(|cursor| Cursor {
                id: cursor.id,
                taken_at: cursor.taken_at.and_utc(),
            }),
        },
    };
    response_headers(Json(body).into_response(), &request_id)
}

async fn refused(
    ctx: &AppContext,
    actor: Option<&HistoryActor>,
    request_id: &str,
    response: Response,
) -> Response {
    match deny_personal_history(&ctx.db, actor, request_id).await {
        Ok(()) => response,
        Err(error) => operation(error, request_id),
    }
}
