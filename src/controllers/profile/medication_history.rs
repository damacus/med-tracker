use axum::{
    body::{Body, Bytes},
    extract::{Query, State, rejection::QueryRejection},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use loco_rs::prelude::{AppContext, TeraView, ViewEngine, format};
use sea_orm::{DatabaseTransaction, IsolationLevel};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

use crate::{
    controllers::{
        medication_history::{Entry, Input},
        medications::{authentication_error, operation_error},
    },
    models::{
        care::{
            account_clinical_access::{AccessDenial, BrowserAccess},
            report_pdf,
            take_history::self_history,
        },
        errors::OperationError,
        identity::{browser, resource::AuthenticationError},
    },
};

pub(super) fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn next_page(input: &Input, cursor: self_history::Cursor) -> String {
    let mut parameters = url::form_urlencoded::Serializer::new(String::new());
    parameters.append_pair("after", &cursor.to_string());
    if let Some(value) = &input.from {
        parameters.append_pair("from", value);
    }
    if let Some(value) = &input.to {
        parameters.append_pair("to", value);
    }
    if let Some(value) = input.limit {
        parameters.append_pair("limit", &value.to_string());
    }
    format!("/profile/medication-history?{}", parameters.finish())
}

pub(super) async fn show(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    headers: HeaderMap,
    input: Result<Query<Input>, QueryRejection>,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    let principal =
        match browser::authenticate(&ctx.db, &session, ctx.config.settings.as_ref()).await {
            Ok(value) => value,
            Err(error) => return authentication_error(error),
        };
    let transaction = match principal
        .clinical_history_transaction(&ctx.db, IsolationLevel::ReadCommitted)
        .await
    {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let prepared = async {
        let Query(input) = input.map_err(|_| {
            crate::models::profile::validation("history", "contains invalid query parameters")
        })?;
        let read_query = self_history::Query::parse(
            input.after.as_deref(),
            input.from.as_deref(),
            input.to.as_deref(),
            input.limit,
        )?;
        let access = BrowserAccess::capture(&transaction, &principal, uuid::Uuid::new_v4()).await?;
        let page = self_history::read(&transaction, &read_query).await?;
        let next = page.next_cursor.map(|cursor| next_page(&input, cursor));
        let entries = page
            .entries
            .into_iter()
            .map(Entry::from)
            .collect::<Vec<_>>();
        Ok::<_, OperationError>((access, entries, next))
    }
    .await;
    let (access, entries, next) = match prepared {
        Ok(value) => value,
        Err(error) => {
            rollback(transaction).await;
            return operation_error(error);
        }
    };
    if let Err(error) = transaction.commit().await {
        return operation_error(error.into());
    }
    let rendered = render_page(&view, &headers, entries, next);
    let rendered = match rendered {
        Ok(value) => value,
        Err(error) => return operation_error(error),
    };
    let release = match principal
        .clinical_history_transaction(&ctx.db, IsolationLevel::ReadCommitted)
        .await
    {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let audit = async {
        access.recheck(&release, &principal).await?;
        access.record_read(&release).await
    }
    .await;
    if let Err(error) = audit {
        rollback(release).await;
        return operation_error(error);
    }
    if let Err(error) = release.commit().await {
        return operation_error(error.into());
    }
    rendered
}

fn render_page(
    view: &TeraView,
    headers: &HeaderMap,
    entries: Vec<Entry>,
    next: Option<String>,
) -> Result<Response, OperationError> {
    let locale = report_pdf::locale(
        headers
            .get(header::ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("en"),
    );
    let labels = report_pdf::translations(locale).map_err(|_| OperationError::Unavailable)?;
    let mut context = crate::controllers::medications::rendering::appearance_context();
    context["title"] = labels["profiles"]["medication_history"]["title"].clone();
    context["lang"] = locale.into();
    context["slug"] = "".into();
    context["labels"] = labels;
    context["entries"] = serde_json::to_value(entries).map_err(serialization_error)?;
    context["next_page"] = next.into();
    format::render()
        .view(
            view,
            "profile/medication-history.html",
            crate::views::template_data(&context),
        )
        .map_err(|_| OperationError::Unavailable)
}

pub(super) async fn download(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
) -> Response {
    let principal =
        match browser::authenticate(&ctx.db, &session, ctx.config.settings.as_ref()).await {
            Ok(value) => value,
            Err(error) => return download_authentication_error(error),
        };
    let transaction = match principal
        .clinical_history_transaction(&ctx.db, IsolationLevel::RepeatableRead)
        .await
    {
        Ok(value) => value,
        Err(error) => return download_authentication_error(error),
    };
    let prepared = async {
        let access = BrowserAccess::capture(&transaction, &principal, uuid::Uuid::new_v4()).await?;
        let file = spool(&transaction).await?;
        Ok::<_, OperationError>((access, file))
    }
    .await;
    let (access, file) = match prepared {
        Ok(value) => value,
        Err(error) => {
            rollback(transaction).await;
            return operation_error(error);
        }
    };
    if let Err(error) = transaction.commit().await {
        return operation_error(error.into());
    }
    let release = match principal
        .clinical_history_transaction(&ctx.db, IsolationLevel::ReadCommitted)
        .await
    {
        Ok(value) => value,
        Err(error) => {
            let denial = match &error {
                AuthenticationError::Unauthenticated => Some(AccessDenial::Unauthenticated),
                AuthenticationError::Forbidden | AuthenticationError::InsufficientScope { .. } => {
                    Some(AccessDenial::Forbidden)
                }
                AuthenticationError::Unavailable => None,
            };
            if let Some(denial) = denial {
                if let Err(error) = access.record_denial(&ctx.db, denial).await {
                    return operation_error(error);
                }
            }
            return download_authentication_error(error);
        }
    };
    let audit = async {
        access.recheck(&release, &principal).await?;
        access.record_download(&release).await
    }
    .await;
    if let Err(error) = audit {
        rollback(release).await;
        return operation_error(error);
    }
    if let Err(error) = release.commit().await {
        return operation_error(error.into());
    }
    stream(file)
}

async fn spool(transaction: &DatabaseTransaction) -> Result<tokio::fs::File, OperationError> {
    let mut file = tokio::fs::File::from_std(tempfile::tempfile().map_err(io_error)?);
    file.write_all(
        b"{\"format\":\"medtracker.personal_medication_history\",\"version\":1,\"exported_at\":",
    )
    .await
    .map_err(io_error)?;
    let exported = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true);
    file.write_all(&serde_json::to_vec(&exported).map_err(serialization_error)?)
        .await
        .map_err(io_error)?;
    file.write_all(b",\"data\":[").await.map_err(io_error)?;
    let mut query = self_history::Query::parse(None, None, None, Some(100))?;
    let mut first = true;
    loop {
        let page = self_history::read(transaction, &query).await?;
        for entry in page.entries {
            if !first {
                file.write_all(b",").await.map_err(io_error)?;
            }
            let entry = Entry::from(entry);
            file.write_all(&serde_json::to_vec(&entry).map_err(serialization_error)?)
                .await
                .map_err(io_error)?;
            first = false;
        }
        let Some(cursor) = page.next_cursor else {
            break;
        };
        query.continue_after(cursor);
    }
    file.write_all(b"]}").await.map_err(io_error)?;
    file.flush().await.map_err(io_error)?;
    file.rewind().await.map_err(io_error)?;
    Ok(file)
}

fn stream(file: tokio::fs::File) -> Response {
    let stream = futures_util::stream::try_unfold(file, |mut file| async move {
        let mut buffer = vec![0_u8; 64 * 1024];
        let count = file.read(&mut buffer).await?;
        if count == 0 {
            return Ok::<_, std::io::Error>(None);
        }
        buffer.truncate(count);
        Ok(Some((Bytes::from(buffer), file)))
    });
    (
        [
            (header::CONTENT_TYPE, "application/json"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"medtracker-medication-history-v1.json\"",
            ),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        Body::from_stream(stream),
    )
        .into_response()
}

async fn rollback(transaction: DatabaseTransaction) {
    if let Err(error) = transaction.rollback().await {
        let _ = OperationError::from(error);
    }
}

fn io_error(error: std::io::Error) -> OperationError {
    tracing::warn!(category=?error.kind(),"Private medication history file operation failed");
    OperationError::Unavailable
}

fn serialization_error(error: serde_json::Error) -> OperationError {
    tracing::warn!(category=?error.classify(),"Medication history presentation failed");
    OperationError::Unavailable
}

fn download_authentication_error(error: AuthenticationError) -> Response {
    match error {
        AuthenticationError::Unauthenticated => {
            (StatusCode::UNAUTHORIZED, "Authentication required").into_response()
        }
        AuthenticationError::Forbidden | AuthenticationError::InsufficientScope { .. } => {
            (StatusCode::FORBIDDEN, "Access denied").into_response()
        }
        AuthenticationError::Unavailable => operation_error(OperationError::Unavailable),
    }
}
