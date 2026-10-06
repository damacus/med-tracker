use axum::http::{HeaderMap, header};
use headers::{Authorization, HeaderMapExt, authorization::Bearer};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, EntityTrait,
    QueryFilter, QueryOrder, Statement, TransactionTrait,
    sea_query::{Expr, ExprTrait},
};
use sha2::{Digest, Sha256};

use super::resource::{self, AuthenticationError};
use crate::models::{
    access::{self, Actor, HouseholdScope},
    care::doses::{CredentialMethod, CredentialProvenance},
    entities::{account, api_app_token, api_session, membership, person},
    errors::OperationError,
};

pub struct ApiSessionPrincipal(StoredPrincipal);

pub struct UserSessionActor {
    pub account_id: i64,
    pub user_id: i64,
    pub person_id: i64,
    pub session_id: i64,
    pub email: String,
}

#[derive(Clone, Copy)]
enum Kind {
    Session,
    App,
}

pub(super) struct StoredPrincipal {
    account_id: i64,
    id: i64,
    digest: String,
    kind: Kind,
}

pub(super) struct StoredAuthority {
    pub account_id: i64,
    pub household_id: i64,
    pub user_id: i64,
    pub person_id: i64,
    pub email: String,
    pub preferences: serde_json::Value,
}

pub async fn authenticate(
    db: &DatabaseConnection,
    headers: &HeaderMap,
) -> Result<ApiSessionPrincipal, AuthenticationError> {
    match authenticate_stored(db, headers).await? {
        Some((principal, _)) if matches!(principal.kind, Kind::Session) => {
            Ok(ApiSessionPrincipal(principal))
        }
        Some(_) => Err(AuthenticationError::Forbidden),
        None => match resource::authenticate_oauth(db, headers).await {
            Ok(_)
            | Err(AuthenticationError::InsufficientScope { .. })
            | Err(AuthenticationError::Forbidden) => Err(AuthenticationError::Forbidden),
            Err(error) => Err(error),
        },
    }
}

pub(super) async fn authenticate_stored(
    db: &DatabaseConnection,
    headers: &HeaderMap,
) -> Result<Option<(StoredPrincipal, StoredAuthority)>, AuthenticationError> {
    if headers.get_all(header::AUTHORIZATION).iter().count() != 1 {
        return Err(AuthenticationError::Unauthenticated);
    }
    let bearer = headers
        .typed_try_get::<Authorization<Bearer>>()
        .map_err(|_| AuthenticationError::Unauthenticated)?
        .ok_or(AuthenticationError::Unauthenticated)?;
    let digest = hex::encode(Sha256::digest(bearer.token().as_bytes()));
    let transaction = db.begin().await.map_err(unavailable)?;
    let result = async {
        transaction.execute_unprepared("SET LOCAL ROLE med_tracker_app; SET LOCAL search_path = pg_catalog, public, pg_temp; SET LOCAL lock_timeout = '5s'; SET LOCAL statement_timeout = '30s'").await.map_err(unavailable)?;
        transaction.execute_unprepared("SELECT set_config('med_tracker.current_account_id', '', true), set_config('med_tracker.current_household_id', '', true), set_config('med_tracker.current_membership_id', '', true), set_config('med_tracker.current_invitation_token_digest', '', true)").await.map_err(unavailable)?;
        let principal = if let Some(row) = api_session::Entity::find()
            .filter(api_session::Column::AccessTokenDigest.eq(&digest))
            .one(&transaction).await.map_err(unavailable)?
        {
            StoredPrincipal { account_id: row.account_id, id: row.id, digest, kind: Kind::Session }
        } else if let Some(row) = api_app_token::Entity::find()
            .filter(api_app_token::Column::TokenDigest.eq(&digest))
            .one(&transaction).await.map_err(unavailable)?
        {
            StoredPrincipal { account_id: row.account_id, id: row.id, digest, kind: Kind::App }
        } else {
            return Ok(None);
        };
        let actor = principal.revalidate(&transaction).await?;
        let update = match principal.kind {
            Kind::Session => "UPDATE api_sessions SET last_used_at=timezone('UTC',clock_timestamp()), updated_at=timezone('UTC',clock_timestamp()) WHERE id=$1",
            Kind::App => "UPDATE api_app_tokens SET last_used_at=timezone('UTC',clock_timestamp()), updated_at=timezone('UTC',clock_timestamp()) WHERE id=$1",
        };
        transaction.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres, update, [principal.id.into()])).await.map_err(unavailable)?;
        Ok(Some((principal, actor)))
    }.await;
    match result {
        Ok(principal) => {
            transaction.commit().await.map_err(unavailable)?;
            Ok(principal)
        }
        Err(error) => {
            transaction.rollback().await.map_err(unavailable)?;
            Err(error)
        }
    }
}

impl ApiSessionPrincipal {
    pub async fn revalidate(
        &self,
        transaction: &DatabaseTransaction,
    ) -> Result<UserSessionActor, AuthenticationError> {
        let actor = self.0.revalidate(transaction).await?;
        Ok(UserSessionActor {
            account_id: actor.account_id,
            user_id: actor.user_id,
            person_id: actor.person_id,
            session_id: self.0.id,
            email: actor.email,
        })
    }
}

impl StoredPrincipal {
    pub(super) fn provenance(&self) -> CredentialProvenance {
        CredentialProvenance {
            method: match self.kind {
                Kind::Session => CredentialMethod::ApiSession,
                Kind::App => CredentialMethod::ApiAppToken,
            },
            reference: self.id.to_string(),
        }
    }

    pub(super) async fn revalidate(
        &self,
        transaction: &DatabaseTransaction,
    ) -> Result<StoredAuthority, AuthenticationError> {
        let (membership_id, version) = match self.kind {
            Kind::Session => {
                let row = api_session::Entity::find_by_id(self.id)
                    .filter(api_session::Column::AccountId.eq(self.account_id))
                    .filter(api_session::Column::AccessTokenDigest.eq(&self.digest))
                    .filter(api_session::Column::RevokedAt.is_null())
                    .filter(
                        Expr::col(api_session::Column::AccessExpiresAt)
                            .gt(Expr::cust("timezone('UTC',clock_timestamp())")),
                    )
                    .one(transaction)
                    .await
                    .map_err(unavailable)?
                    .ok_or(AuthenticationError::Unauthenticated)?;
                (
                    row.household_membership_id
                        .ok_or(AuthenticationError::Unauthenticated)?,
                    row.permissions_version,
                )
            }
            Kind::App => {
                let row = api_app_token::Entity::find_by_id(self.id)
                    .filter(api_app_token::Column::AccountId.eq(self.account_id))
                    .filter(api_app_token::Column::TokenDigest.eq(&self.digest))
                    .filter(api_app_token::Column::RevokedAt.is_null())
                    .filter(
                        Expr::col(api_app_token::Column::ExpiresAt)
                            .gt(Expr::cust("timezone('UTC',clock_timestamp())")),
                    )
                    .one(transaction)
                    .await
                    .map_err(unavailable)?
                    .ok_or(AuthenticationError::Unauthenticated)?;
                app_lifetime(transaction, row.created_at).await?;
                (row.household_membership_id, row.permissions_version)
            }
        };
        authority(transaction, self.account_id, membership_id, version).await
    }
}

async fn app_lifetime(
    transaction: &DatabaseTransaction,
    created_at: chrono::NaiveDateTime,
) -> Result<(), AuthenticationError> {
    let months = match std::env::var("API_APP_TOKEN_MAX_AGE_MONTHS") {
        Ok(value) => value.parse::<u32>().ok().filter(|months| *months > 0),
        Err(std::env::VarError::NotPresent) => Some(12),
        Err(std::env::VarError::NotUnicode(_)) => None,
    }
    .ok_or(AuthenticationError::Unauthenticated)?;
    let maximum = created_at
        .checked_add_months(chrono::Months::new(months))
        .ok_or(AuthenticationError::Unauthenticated)?;
    let current = transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT $1::timestamp > timezone('UTC',clock_timestamp()) AS eligible",
            [maximum.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(AuthenticationError::Unavailable)?;
    if current
        .try_get::<bool>("", "eligible")
        .map_err(unavailable)?
    {
        Ok(())
    } else {
        Err(AuthenticationError::Unauthenticated)
    }
}

async fn authority(
    transaction: &DatabaseTransaction,
    account_id: i64,
    membership_id: i64,
    permissions_version: i32,
) -> Result<StoredAuthority, AuthenticationError> {
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT set_config('med_tracker.current_account_id',$1,true)",
            [account_id.to_string().into()],
        ))
        .await
        .map_err(unavailable)?;
    let source = membership::Entity::find_by_id(membership_id)
        .filter(membership::Column::AccountId.eq(account_id))
        .one(transaction)
        .await
        .map_err(unavailable)?
        .ok_or(AuthenticationError::Unauthenticated)?;
    let (current, user_id) = access::verify_membership(
        transaction,
        &HouseholdScope {
            actor: Actor { account_id },
            household_id: source.household_id,
            request_id: String::new(),
        },
    )
    .await
    .map_err(|error| match error {
        OperationError::Unavailable => AuthenticationError::Unavailable,
        _ => AuthenticationError::Unauthenticated,
    })?;
    if current.id != membership_id || current.permissions_version != permissions_version {
        return Err(AuthenticationError::Unauthenticated);
    }
    let person = person::Entity::find()
        .filter(person::Column::AccountId.eq(account_id))
        .order_by_asc(person::Column::Id)
        .one(transaction)
        .await
        .map_err(unavailable)?
        .ok_or(AuthenticationError::Unauthenticated)?;
    let account = account::Entity::find_by_id(account_id)
        .one(transaction)
        .await
        .map_err(unavailable)?
        .ok_or(AuthenticationError::Unauthenticated)?;
    Ok(StoredAuthority {
        account_id,
        household_id: current.household_id,
        user_id,
        person_id: person.id,
        email: account.email,
        preferences: account.preferences,
    })
}

fn unavailable(_: sea_orm::DbErr) -> AuthenticationError {
    AuthenticationError::Unavailable
}
