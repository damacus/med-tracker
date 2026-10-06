use axum::{body::Body, http::Request};
use oxide_auth_async::primitives::Registrar;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait, QueryFilter,
    Statement, TransactionTrait,
};

use super::{ExchangeError, input::Input, registrar::RegisteredClient, store::digest};
use crate::models::entities::{oauth_application, oauth_grant};

pub async fn revoke(db: &DatabaseConnection, request: Request<Body>) -> Result<(), ExchangeError> {
    let request_id = request
        .extensions()
        .get::<loco_rs::controller::middleware::request_id::LocoRequestId>()
        .map(|id| id.get().to_owned());
    let input = Input::parse(request).await?;
    let token = input.value("token").ok_or(ExchangeError::InvalidRequest)?;
    let transaction = db.begin().await.map_err(|_| ExchangeError::Unavailable)?;
    transaction.execute_unprepared("SET LOCAL ROLE med_tracker_app; SET LOCAL search_path = pg_catalog, public, pg_temp; SET LOCAL lock_timeout = '5s'; SET LOCAL statement_timeout = '30s'").await.map_err(|_| ExchangeError::Unavailable)?;
    let application = oauth_application::Entity::find()
        .filter(oauth_application::Column::ClientId.eq(&input.id))
        .one(&transaction)
        .await
        .map_err(|_| ExchangeError::Unavailable)?
        .ok_or(ExchangeError::InvalidClient)?;
    let id = application.id;
    RegisteredClient::new(application, &input)?
        .check(&input.id, input.secret.as_deref().map(str::as_bytes))
        .await
        .map_err(|_| ExchangeError::InvalidClient)?;
    let revoked = transaction.query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE oauth_grants SET revoked_at=clock_timestamp(), updated_at=clock_timestamp() WHERE oauth_application_id=$1 AND revoked_at IS NULL AND (token_hash=$2 OR refresh_token_hash=$2) RETURNING id", [id.into(), digest(&token).into()])).await.map_err(|_| ExchangeError::Unavailable)?;
    for row in revoked {
        let id: i64 = row
            .try_get("", "id")
            .map_err(|_| ExchangeError::Unavailable)?;
        let grant = oauth_grant::Entity::find_by_id(id)
            .one(&transaction)
            .await
            .map_err(|_| ExchangeError::Unavailable)?
            .ok_or(ExchangeError::Unavailable)?;
        super::audit::record(&transaction, &grant, "token_revoked", request_id.as_deref()).await?;
    }
    transaction
        .commit()
        .await
        .map_err(|_| ExchangeError::Unavailable)
}

pub async fn mobile_clients(
    db: &DatabaseConnection,
) -> Result<Vec<serde_json::Value>, ExchangeError> {
    let clients = oauth_application::Entity::find()
        .filter(oauth_application::Column::ClientKind.eq("mobile"))
        .all(db)
        .await
        .map_err(|_| ExchangeError::Unavailable)?;
    Ok(clients.into_iter().map(|client| serde_json::json!({"client_id":client.client_id,"redirect_uris":client.redirect_uri.split_whitespace().collect::<Vec<_>>(),"scopes":client.scopes.split_whitespace().collect::<Vec<_>>()})).collect())
}
