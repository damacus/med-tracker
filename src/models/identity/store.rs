use async_trait::async_trait;
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE, URL_SAFE_NO_PAD},
};
use chrono::{Duration, Utc};
use oxide_auth::{
    frontends::simple::extensions::Pkce,
    primitives::{
        generator::{RandomGenerator, TagGrant},
        grant::{Extensions, Grant},
        issuer::{IssuedToken, RefreshedToken, TokenType},
    },
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseTransaction, DbBackend, EntityTrait, QueryFilter,
    QuerySelect, Set, Statement,
};
use sha2::{Digest, Sha256};

use crate::models::{
    access::{self, Actor, HouseholdScope},
    entities::{membership, oauth_grant, person},
    errors::OperationError,
};

pub(super) fn digest(token: &str) -> String {
    URL_SAFE.encode(Sha256::digest(token.as_bytes()))
}

pub(super) fn pkce(mobile: bool) -> Pkce {
    if mobile {
        Pkce::required()
    } else {
        let mut pkce = Pkce::required();
        pkce.allow_plain();
        pkce
    }
}

pub(super) struct Store<'a> {
    pub transaction: &'a DatabaseTransaction,
    pub application_id: i64,
    pub client_id: String,
    pub redirect: Option<String>,
    pub selected: Option<oauth_grant::Model>,
    pub lifetime: Lifetime,
}

pub struct Lifetime {
    pub(super) inactivity: Duration,
    pub(super) maximum_age: Option<Duration>,
}

impl Lifetime {
    pub fn new(inactivity_days: i64, maximum_age_days: i64) -> Result<Self, super::ExchangeError> {
        if inactivity_days < 1 || maximum_age_days < 0 {
            return Err(super::ExchangeError::Unavailable);
        }
        let inactivity =
            Duration::try_days(inactivity_days).ok_or(super::ExchangeError::Unavailable)?;
        let maximum =
            Duration::try_days(maximum_age_days).ok_or(super::ExchangeError::Unavailable)?;
        Ok(Self {
            inactivity,
            maximum_age: (maximum != Duration::zero()).then_some(maximum),
        })
    }

    pub fn inactivity_days(&self) -> i64 {
        self.inactivity.num_days()
    }

    pub fn maximum_age_days(&self) -> i64 {
        self.maximum_age.map_or(0, |duration| duration.num_days())
    }

    pub fn from_environment() -> Result<Self, super::ExchangeError> {
        fn days(name: &str, default: i64, maximum: i64) -> Result<i64, super::ExchangeError> {
            match std::env::var(name) {
                Ok(value) => match value.parse::<i64>() {
                    Ok(configured) if (1..=maximum).contains(&configured) => Ok(configured),
                    _ => Err(super::ExchangeError::Unavailable),
                },
                Err(std::env::VarError::NotPresent) => Ok(default),
                Err(_) => Err(super::ExchangeError::Unavailable),
            }
        }
        Self::new(
            days("SESSION_INACTIVITY_TIMEOUT_DAYS", 7, 7)?,
            days("SESSION_MAX_AGE_DAYS", 30, 30)?,
        )
    }
}

impl Store<'_> {
    pub(super) async fn authority(&self, row: &oauth_grant::Model) -> Result<bool, ()> {
        self.transaction
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT set_config('med_tracker.current_account_id', $1, true)",
                [row.account_id.to_string().into()],
            ))
            .await
            .map_err(|_| ())?;
        if row.client_kind == "mobile" {
            return self.mobile_authority(row).await;
        }
        if row.client_kind != "integration" {
            return Ok(false);
        }
        let Some(member_id) = row.household_membership_id else {
            return Ok(false);
        };
        let Some(member) = membership::Entity::find_by_id(member_id)
            .one(self.transaction)
            .await
            .map_err(|_| ())?
        else {
            return Ok(false);
        };
        let scope = HouseholdScope {
            actor: Actor {
                account_id: row.account_id,
            },
            household_id: member.household_id,
            request_id: String::new(),
        };
        let checked = match access::verify_membership(self.transaction, &scope).await {
            Ok((checked, _)) => checked,
            Err(OperationError::Unavailable) => return Err(()),
            Err(_) => return Ok(false),
        };
        if checked.id != member_id || Some(checked.permissions_version) != row.permissions_version {
            return Ok(false);
        }
        self.transaction.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT set_config('med_tracker.current_household_id', $1, true), set_config('med_tracker.current_membership_id', $2, true)",
            [member.household_id.to_string().into(), member.id.to_string().into()])).await.map_err(|_| ())?;
        let Some(person_id) = row.person_id else {
            return Ok(false);
        };
        Ok(person::Entity::find_by_id(person_id)
            .filter(person::Column::HouseholdId.eq(member.household_id))
            .one(self.transaction)
            .await
            .map_err(|_| ())?
            .is_some())
    }

    async fn mobile_authority(&self, row: &oauth_grant::Model) -> Result<bool, ()> {
        if row.household_membership_id.is_some()
            || row.person_id.is_some()
            || row.permissions_version.is_some()
            || !row
                .scopes
                .split_whitespace()
                .any(|scope| scope == "medtracker")
        {
            return Ok(false);
        }
        match access::verify_account_actor(self.transaction, row.account_id).await {
            Ok(_) => (),
            Err(OperationError::Unavailable) => return Err(()),
            Err(_) => return Ok(false),
        }
        let (Some(authenticated), Some(last_used)) = (row.authenticated_at, row.last_used_at)
        else {
            return Ok(false);
        };
        let now: chrono::NaiveDateTime = self
            .transaction
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT timezone('UTC', clock_timestamp()) AS now",
            ))
            .await
            .map_err(|_| ())?
            .ok_or(())?
            .try_get("", "now")
            .map_err(|_| ())?;
        let inactivity_deadline = last_used
            .checked_add_signed(self.lifetime.inactivity)
            .ok_or(())?;
        let maximum_valid = match self.lifetime.maximum_age {
            Some(maximum) => authenticated.checked_add_signed(maximum).ok_or(())? > now,
            None => true,
        };
        Ok(inactivity_deadline > now && maximum_valid)
    }

    fn grant(&self, row: &oauth_grant::Model, refresh: bool) -> Result<Grant, ()> {
        let mut extensions = Extensions::new();
        if !refresh {
            let pkce = pkce(row.client_kind == "mobile");
            if let Some(value) = pkce.challenge(
                row.code_challenge_method.as_deref().map(Into::into),
                row.code_challenge.as_deref().map(Into::into),
            )? {
                extensions.set(&pkce, value);
            }
        }
        let until = if refresh {
            let lifetime = if row.client_kind == "mobile" {
                self.lifetime.inactivity
            } else {
                Duration::days(30)
            };
            let mut until = row
                .expires_in
                .checked_add_signed(lifetime - Duration::seconds(900))
                .ok_or(())?;
            if row.client_kind == "mobile" {
                until = until.min(
                    row.last_used_at
                        .ok_or(())?
                        .checked_add_signed(self.lifetime.inactivity)
                        .ok_or(())?,
                );
                if let Some(maximum) = self.lifetime.maximum_age {
                    until = until.min(
                        row.authenticated_at
                            .ok_or(())?
                            .checked_add_signed(maximum)
                            .ok_or(())?,
                    );
                }
            }
            until
        } else {
            row.expires_in
        };
        Ok(Grant {
            owner_id: row.account_id.to_string(),
            client_id: self.client_id.clone(),
            scope: row.scopes.parse().map_err(|_| ())?,
            redirect_uri: row
                .redirect_uri
                .as_ref()
                .ok_or(())?
                .parse()
                .map_err(|_| ())?,
            until: until.and_utc(),
            extensions,
        })
    }

    async fn recover(
        &mut self,
        column: oauth_grant::Column,
        value: &str,
        refresh: bool,
    ) -> Result<Option<Grant>, ()> {
        let mut query = oauth_grant::Entity::find()
            .filter(oauth_grant::Column::OauthApplicationId.eq(self.application_id))
            .filter(column.eq(value))
            .filter(oauth_grant::Column::RevokedAt.is_null());
        if !refresh {
            let Some(redirect) = &self.redirect else {
                return Ok(None);
            };
            query = query.filter(oauth_grant::Column::RedirectUri.eq(redirect));
        }
        let Some(row) = query
            .lock_exclusive()
            .one(self.transaction)
            .await
            .map_err(|_| ())?
        else {
            return Ok(None);
        };
        if !self.authority(&row).await? {
            return Ok(None);
        }
        let Ok(grant) = self.grant(&row, refresh) else {
            return Ok(None);
        };
        self.selected = Some(row);
        Ok(Some(grant))
    }

    async fn issue_tokens(&mut self, grant: Grant) -> Result<IssuedToken, ()> {
        let row = self.selected.as_ref().ok_or(())?;
        if grant.owner_id != row.account_id.to_string() || grant.client_id != self.client_id {
            return Err(());
        }
        let mut generator = RandomGenerator::new(32);
        let token =
            URL_SAFE_NO_PAD.encode(STANDARD.decode(generator.tag(0, &grant)?).map_err(|_| ())?);
        let refresh =
            URL_SAFE_NO_PAD.encode(STANDARD.decode(generator.tag(1, &grant)?).map_err(|_| ())?);
        let until = Utc::now() + Duration::seconds(900);
        let changes = oauth_grant::ActiveModel {
            code: Set(None),
            token_hash: Set(Some(digest(&token))),
            refresh_token_hash: Set(Some(digest(&refresh))),
            expires_in: Set(until.naive_utc()),
            access_token_scopes: Set(Some(grant.scope.to_string())),
            access_token_scope_hash: Set(Some(digest(&token))),
            updated_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        };
        let result = oauth_grant::Entity::update_many()
            .set(changes)
            .filter(oauth_grant::Column::Id.eq(row.id))
            .filter(oauth_grant::Column::RevokedAt.is_null())
            .exec(self.transaction)
            .await
            .map_err(|_| ())?;
        if result.rows_affected != 1 {
            return Err(());
        }
        Ok(IssuedToken {
            token,
            refresh: Some(refresh),
            until,
            token_type: TokenType::Bearer,
        })
    }
}

#[async_trait]
impl oxide_auth_async::primitives::Authorizer for Store<'_> {
    async fn authorize(&mut self, _: Grant) -> Result<String, ()> {
        Err(())
    }

    async fn extract(&mut self, code: &str) -> Result<Option<Grant>, ()> {
        let grant = self.recover(oauth_grant::Column::Code, code, false).await?;
        if grant.is_some() {
            oauth_grant::Entity::update_many()
                .col_expr(
                    oauth_grant::Column::Code,
                    sea_orm::sea_query::Expr::value(Option::<String>::None),
                )
                .filter(oauth_grant::Column::Id.eq(self.selected.as_ref().ok_or(())?.id))
                .exec(self.transaction)
                .await
                .map_err(|_| ())?;
        }
        Ok(grant)
    }
}

#[async_trait]
impl oxide_auth_async::primitives::Issuer for Store<'_> {
    async fn issue(&mut self, grant: Grant) -> Result<IssuedToken, ()> {
        self.issue_tokens(grant).await
    }

    async fn refresh(&mut self, token: &str, grant: Grant) -> Result<RefreshedToken, ()> {
        if self
            .selected
            .as_ref()
            .and_then(|row| row.refresh_token_hash.as_deref())
            != Some(digest(token).as_str())
        {
            return Err(());
        }
        let issued = self.issue_tokens(grant).await?;
        Ok(RefreshedToken {
            token: issued.token,
            refresh: issued.refresh,
            until: issued.until,
            token_type: issued.token_type,
        })
    }

    async fn recover_token(&mut self, _: &str) -> Result<Option<Grant>, ()> {
        Err(())
    }

    async fn recover_refresh(&mut self, token: &str) -> Result<Option<Grant>, ()> {
        self.recover(oauth_grant::Column::RefreshTokenHash, &digest(token), true)
            .await
    }
}
