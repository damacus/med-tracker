use std::borrow::Cow;

use async_trait::async_trait;
use axum::http::{HeaderMap, header};
use chrono::Utc;
use oxide_auth::primitives::{
    grant::{Extensions, Grant},
    issuer::{IssuedToken, RefreshedToken},
    scope::Scope,
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction, EntityTrait,
    QueryFilter, QuerySelect, TransactionTrait,
};

use super::store::{self, Lifetime, Store};
use crate::models::{
    access::{self, Actor, HouseholdScope, TenantTransaction},
    care::doses::{CredentialMethod, CredentialProvenance},
    entities::{account, oauth_application, oauth_grant},
    errors::OperationError,
};

#[derive(Debug, PartialEq)]
pub enum AuthenticationError {
    Unauthenticated,
    Forbidden,
    InsufficientScope { authenticate: String },
    Unavailable,
}

pub struct ValidatedPrincipal {
    account_id: i64,
    provenance: CredentialProvenance,
    time_zone: chrono_tz::Tz,
    authorization: String,
    grant_id: i64,
}

impl ValidatedPrincipal {
    pub fn account_id(&self) -> i64 {
        self.account_id
    }

    pub fn provenance(&self) -> &CredentialProvenance {
        &self.provenance
    }

    pub fn time_zone(&self) -> chrono_tz::Tz {
        self.time_zone
    }

    pub async fn begin_household(
        &self,
        db: &DatabaseConnection,
        household_id: i64,
        request_id: String,
    ) -> Result<TenantTransaction, AuthenticationError> {
        let scope = HouseholdScope {
            actor: Actor {
                account_id: self.account_id,
            },
            household_id,
            request_id,
        };
        let tenant = access::begin(db, &scope).await.map_err(operation_error)?;
        match validate(tenant.transaction(), &self.authorization).await {
            Ok(row) if row.id == self.grant_id && row.account_id == self.account_id => Ok(tenant),
            result => {
                tenant.rollback().await.map_err(operation_error)?;
                Err(result.err().unwrap_or(AuthenticationError::Unauthenticated))
            }
        }
    }
}

pub async fn authenticate(
    db: &DatabaseConnection,
    headers: &HeaderMap,
) -> Result<ValidatedPrincipal, AuthenticationError> {
    let mut values = headers.get_all(header::AUTHORIZATION).iter();
    let authorization = values
        .next()
        .and_then(|value| value.to_str().ok())
        .ok_or(AuthenticationError::Unauthenticated)?;
    if values.next().is_some() {
        return Err(AuthenticationError::Unauthenticated);
    }
    let transaction = db
        .begin()
        .await
        .map_err(|_| AuthenticationError::Unavailable)?;
    let result = async {
        transaction.execute_unprepared("SET LOCAL ROLE med_tracker_app; SET LOCAL search_path = pg_catalog, public, pg_temp; SET LOCAL lock_timeout = '5s'; SET LOCAL statement_timeout = '30s'").await.map_err(|_| AuthenticationError::Unavailable)?;
        transaction.execute_unprepared("SELECT set_config('med_tracker.current_account_id', '', true), set_config('med_tracker.current_household_id', '', true), set_config('med_tracker.current_membership_id', '', true), set_config('med_tracker.current_invitation_token_digest', '', true)").await.map_err(|_| AuthenticationError::Unavailable)?;
        let row = validate(&transaction, authorization).await?;
        let account = account::Entity::find_by_id(row.account_id).one(&transaction).await.map_err(|_| AuthenticationError::Unavailable)?.ok_or(AuthenticationError::Unauthenticated)?;
        let time_zone = super::time_zone::preferred(&account.preferences)?;
        Ok(ValidatedPrincipal {
            account_id: row.account_id,
            provenance: CredentialProvenance { method: CredentialMethod::OauthGrant, reference: row.id.to_string() },
            time_zone,
            authorization: authorization.to_owned(),
            grant_id: row.id,
        })
    }.await;
    match result {
        Ok(principal) => {
            transaction
                .commit()
                .await
                .map_err(|_| AuthenticationError::Unavailable)?;
            Ok(principal)
        }
        Err(error) => {
            transaction
                .rollback()
                .await
                .map_err(|_| AuthenticationError::Unavailable)?;
            Err(error)
        }
    }
}

pub(super) fn operation_error(error: OperationError) -> AuthenticationError {
    match error {
        OperationError::Unauthenticated => AuthenticationError::Unauthenticated,
        OperationError::Unavailable => AuthenticationError::Unavailable,
        _ => AuthenticationError::Forbidden,
    }
}

struct ResourceRequest<'a>(&'a str);

impl oxide_auth::code_grant::resource::Request for ResourceRequest<'_> {
    fn valid(&self) -> bool {
        true
    }
    fn token(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(self.0))
    }
}

struct ResourceIssuer<'a> {
    transaction: &'a DatabaseTransaction,
    selected: Option<oauth_grant::Model>,
    scopes: Vec<Scope>,
}

impl oxide_auth_async::code_grant::resource::Endpoint for ResourceIssuer<'_> {
    fn scopes(&mut self) -> &[Scope] {
        &self.scopes
    }
    fn issuer(&mut self) -> &mut (dyn oxide_auth_async::primitives::Issuer + Send) {
        self
    }
}

#[async_trait]
impl oxide_auth_async::primitives::Issuer for ResourceIssuer<'_> {
    async fn issue(&mut self, _: Grant) -> Result<IssuedToken, ()> {
        Err(())
    }
    async fn refresh(&mut self, _: &str, _: Grant) -> Result<RefreshedToken, ()> {
        Err(())
    }
    async fn recover_refresh(&mut self, _: &str) -> Result<Option<Grant>, ()> {
        Err(())
    }
    async fn recover_token(&mut self, token: &str) -> Result<Option<Grant>, ()> {
        let Some(row) = oauth_grant::Entity::find()
            .filter(oauth_grant::Column::TokenHash.eq(store::digest(token)))
            .filter(oauth_grant::Column::RevokedAt.is_null())
            .lock_shared()
            .one(self.transaction)
            .await
            .map_err(|_| ())?
        else {
            return Ok(None);
        };
        if row.client_kind != "mobile" {
            return Ok(None);
        }
        let Some(application) = oauth_application::Entity::find_by_id(row.oauth_application_id)
            .one(self.transaction)
            .await
            .map_err(|_| ())?
        else {
            return Ok(None);
        };
        if application.client_kind != "mobile" {
            return Ok(None);
        }
        let store = Store {
            transaction: self.transaction,
            application_id: application.id,
            client_id: application.client_id.clone(),
            redirect: None,
            selected: None,
            lifetime: Lifetime::from_environment().map_err(|_| ())?,
        };
        if !store.authority(&row).await? {
            return Ok(None);
        }
        let grant = Grant {
            owner_id: row.account_id.to_string(),
            client_id: application.client_id,
            scope: if row.access_token_scope_hash.is_some()
                && row.access_token_scope_hash == row.token_hash
            {
                row.access_token_scopes.as_deref().ok_or(())?
            } else {
                &row.scopes
            }
            .parse()
            .map_err(|_| ())?,
            redirect_uri: row
                .redirect_uri
                .as_deref()
                .ok_or(())?
                .parse()
                .map_err(|_| ())?,
            until: chrono::DateTime::<Utc>::from_naive_utc_and_offset(row.expires_in, Utc),
            extensions: Extensions::new(),
        };
        self.selected = Some(row);
        Ok(Some(grant))
    }
}

async fn validate(
    transaction: &DatabaseTransaction,
    authorization: &str,
) -> Result<oauth_grant::Model, AuthenticationError> {
    let mut endpoint = ResourceIssuer {
        transaction,
        selected: None,
        scopes: vec![
            "medtracker"
                .parse()
                .map_err(|_| AuthenticationError::Unavailable)?,
        ],
    };
    oxide_auth_async::code_grant::resource::protect(&mut endpoint, &ResourceRequest(authorization))
        .await
        .map_err(|error| {
            use oxide_auth::code_grant::resource::{AccessFailure, Error, ErrorCode};
            match error {
                Error::PrimitiveError => AuthenticationError::Unavailable,
                Error::AccessDenied {
                    failure:
                        AccessFailure {
                            code: Some(ErrorCode::InsufficientScope),
                        },
                    ..
                } => AuthenticationError::InsufficientScope {
                    authenticate: error.www_authenticate(),
                },
                _ => AuthenticationError::Unauthenticated,
            }
        })?;
    endpoint.selected.ok_or(AuthenticationError::Unavailable)
}
