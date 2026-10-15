use super::account_profile::{authentication, no_store, operation};
use crate::{
    controllers::medication_history::{Entry, Input},
    models::{
        care::{account_clinical_access, take_history::self_history},
        errors::OperationError,
        identity::resource,
    },
};
use axum::{
    Json,
    extract::{Query, State, rejection::QueryRejection},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use loco_rs::prelude::AppContext;
use serde::Serialize;

#[derive(Serialize)]
struct Metadata {
    next_cursor: Option<String>,
}

#[derive(Serialize)]
struct Body {
    data: Vec<Entry>,
    meta: Metadata,
}

pub(super) async fn show(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    input: Result<Query<Input>, QueryRejection>,
) -> Response {
    let trace = uuid::Uuid::new_v4();
    let request_id = trace.to_string();
    let principal =
        match resource::authenticate(&ctx.db, &headers, ctx.config.settings.as_ref()).await {
            Ok(principal) => principal,
            Err(error) => return authentication(error, &request_id),
        };
    let transaction = match principal
        .global_self_history_transaction(&ctx.db, ctx.config.settings.as_ref())
        .await
    {
        Ok(transaction) => transaction,
        Err(error) => return authentication(error, &request_id),
    };
    let read = async {
        let Query(input) = input.map_err(|_| {
            crate::models::profile::validation("history", "contains invalid query parameters")
        })?;
        let query = self_history::Query::parse(
            input.after.as_deref(),
            input.from.as_deref(),
            input.to.as_deref(),
            input.limit,
        )?;
        let page = self_history::read(&transaction, &query).await?;
        account_clinical_access::record_read(&transaction, principal.provenance(), trace).await?;
        Ok::<_, OperationError>(page)
    }
    .await;
    let page = match read {
        Ok(page) => {
            if let Err(error) = transaction.commit().await {
                return operation(error.into(), &request_id);
            }
            page
        }
        Err(error) => {
            if let Err(rollback_error) = transaction.rollback().await {
                let _ = OperationError::from(rollback_error);
            }
            return operation(error, &request_id);
        }
    };
    no_store(
        Json(Body {
            data: page.entries.into_iter().map(Entry::from).collect(),
            meta: Metadata {
                next_cursor: page.next_cursor.map(|id| id.to_string()),
            },
        })
        .into_response(),
    )
}
