use axum::{body::Body, http::Request};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait, QueryFilter,
    QuerySelect, Statement, TransactionTrait,
};

mod audit;
pub mod authorization;
pub mod browser;
mod input;
pub mod oauth;
mod registrar;
pub mod resource;
mod store;
mod time_zone;

use crate::models::entities::oauth_application;
use input::Input;
use oxide_auth::frontends::simple::extensions::AddonList;
use registrar::RegisteredClient;
pub use store::Lifetime as AuthenticationLifetime;
use store::Store;

pub async fn exchange(
    db: &DatabaseConnection,
    request: Request<Body>,
) -> Result<serde_json::Value, ExchangeError> {
    exchange_with_lifetime(db, request, AuthenticationLifetime::from_environment()?).await
}

#[derive(Debug, PartialEq)]
pub enum ExchangeError {
    InvalidRequest,
    InvalidClient,
    InvalidGrant,
    Unavailable,
    Protocol {
        body: serde_json::Value,
        authenticate: Option<String>,
    },
}

pub async fn exchange_with_lifetime(
    db: &DatabaseConnection,
    request: Request<Body>,
    lifetime: AuthenticationLifetime,
) -> Result<serde_json::Value, ExchangeError> {
    let input = Input::parse(request).await?;
    let transaction = db.begin().await.map_err(|_| ExchangeError::Unavailable)?;
    let result = async {
        transaction.execute_unprepared("SET LOCAL ROLE med_tracker_app; SET LOCAL search_path = pg_catalog, public, pg_temp; SET LOCAL lock_timeout = '5s'; SET LOCAL statement_timeout = '30s'").await.map_err(|_| ExchangeError::Unavailable)?;
        transaction.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT set_config('med_tracker.current_account_id', '', true), set_config('med_tracker.current_household_id', '', true), set_config('med_tracker.current_membership_id', '', true), set_config('med_tracker.current_invitation_token_digest', '', true)"))
            .await.map_err(|_| ExchangeError::Unavailable)?;
        let application = oauth_application::Entity::find().filter(oauth_application::Column::ClientId.eq(&input.id))
            .lock_shared().one(&transaction).await.map_err(|_| ExchangeError::Unavailable)?.ok_or(ExchangeError::InvalidClient)?;
        let mut addons = AddonList::new();
        addons.push_code(store::pkce(application.client_kind == "mobile"));
        let mut endpoint = Endpoint {
            store: Store { transaction: &transaction, application_id: application.id, client_id: application.client_id.clone(), redirect: input.value("redirect_uri").map(|value| value.into_owned()), selected: None, lifetime },
            registrar: RegisteredClient::new(application, &input)?, addons,
        };
        let json = if input.value("grant_type").as_deref() == Some("refresh_token") {
            oxide_auth_async::code_grant::refresh::refresh(&mut endpoint, &input).await.map_err(refresh_error)?.to_json()
        } else {
            oxide_auth_async::code_grant::access_token::access_token(&mut endpoint, &input).await.map_err(code_error)?.to_json()
        };
        let mut response: serde_json::Value = serde_json::from_str(&json).map_err(|_| ExchangeError::Unavailable)?;
        if let Some(person_id) = endpoint.store.selected.as_ref().and_then(|grant| grant.person_id) {
            let person = crate::models::entities::person::Entity::find_by_id(person_id).one(&transaction).await.map_err(|_| ExchangeError::Unavailable)?.ok_or(ExchangeError::InvalidGrant)?;
            response["patient"] = person.portable_id.into();
        }
        Ok(response)
    }.await;
    match result {
        Ok(response) => {
            transaction
                .commit()
                .await
                .map_err(|_| ExchangeError::Unavailable)?;
            Ok(response)
        }
        Err(error) => {
            transaction
                .rollback()
                .await
                .map_err(|_| ExchangeError::Unavailable)?;
            Err(error)
        }
    }
}

struct Endpoint<'a> {
    registrar: RegisteredClient,
    store: Store<'a>,
    addons: AddonList,
}

impl oxide_auth_async::code_grant::access_token::Endpoint for Endpoint<'_> {
    fn registrar(&self) -> &(dyn oxide_auth_async::primitives::Registrar + Sync) {
        &self.registrar
    }
    fn authorizer(&mut self) -> &mut (dyn oxide_auth_async::primitives::Authorizer + Send) {
        &mut self.store
    }
    fn issuer(&mut self) -> &mut (dyn oxide_auth_async::primitives::Issuer + Send) {
        &mut self.store
    }
    fn extension(
        &mut self,
    ) -> &mut (dyn oxide_auth_async::code_grant::access_token::Extension + Send) {
        &mut self.addons
    }
}

impl oxide_auth_async::code_grant::refresh::Endpoint for Endpoint<'_> {
    fn registrar(&self) -> &(dyn oxide_auth_async::primitives::Registrar + Sync) {
        &self.registrar
    }
    fn issuer(&mut self) -> &mut (dyn oxide_auth_async::primitives::Issuer + Send) {
        &mut self.store
    }
}

fn protocol_error(json: String, authenticate: Option<String>) -> ExchangeError {
    match serde_json::from_str(&json) {
        Ok(body) => ExchangeError::Protocol { body, authenticate },
        Err(_) => ExchangeError::Unavailable,
    }
}

fn code_error(error: oxide_auth::code_grant::accesstoken::Error) -> ExchangeError {
    use oxide_auth::code_grant::accesstoken::Error;
    match error {
        Error::Invalid(error) => protocol_error(error.to_json(), None),
        Error::Unauthorized(error, challenge) => protocol_error(error.to_json(), Some(challenge)),
        Error::Primitive(_) => ExchangeError::Unavailable,
    }
}

fn refresh_error(error: oxide_auth::code_grant::refresh::Error) -> ExchangeError {
    use oxide_auth::code_grant::refresh::Error;
    match error {
        Error::Invalid(error) => protocol_error(error.to_json(), None),
        Error::Unauthorized(error, challenge) => protocol_error(error.to_json(), Some(challenge)),
        Error::Primitive => ExchangeError::Unavailable,
    }
}
