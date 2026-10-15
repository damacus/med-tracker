use super::{ExchangeError, browser::BrowserPrincipal, registrar::RegisteredClient, store};
use crate::models::entities::{oauth_application, oauth_grant};
use async_trait::async_trait;
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use oxide_auth::{
    code_grant::authorization::{Error, Request},
    frontends::simple::extensions::AddonList,
    primitives::{
        generator::{RandomGenerator, TagGrant},
        grant::Grant,
        registrar::{ClientUrl, ExactUrl},
    },
};
use oxide_auth_async::{
    code_grant::authorization::{self, Pending},
    primitives::{Authorizer, Registrar},
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, EntityTrait,
    QueryFilter, QuerySelect, Statement,
};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Clone, Deserialize, Serialize)]
pub struct AuthorizationInput(pub Vec<(String, String)>);

impl AuthorizationInput {
    pub fn value(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

impl Request for AuthorizationInput {
    fn valid(&self) -> bool {
        [
            "client_id",
            "redirect_uri",
            "response_type",
            "response_mode",
            "scope",
            "state",
            "code_challenge",
            "code_challenge_method",
        ]
        .iter()
        .all(|name| self.0.iter().filter(|(key, _)| key == name).count() <= 1)
            && self
                .value("response_mode")
                .is_none_or(|value| matches!(value, "query" | "form_post"))
    }
    fn client_id(&self) -> Option<Cow<'_, str>> {
        self.value("client_id").map(Into::into)
    }
    fn scope(&self) -> Option<Cow<'_, str>> {
        self.value("scope").map(Into::into)
    }
    fn redirect_uri(&self) -> Option<Cow<'_, str>> {
        self.value("redirect_uri").map(Into::into)
    }
    fn state(&self) -> Option<Cow<'_, str>> {
        self.value("state").map(Into::into)
    }
    fn response_type(&self) -> Option<Cow<'_, str>> {
        self.value("response_type").map(Into::into)
    }
    fn extension(&self, name: &str) -> Option<Cow<'_, str>> {
        self.value(name).map(Into::into)
    }
}

pub struct Prepared {
    pub application: oauth_application::Model,
    pub pending: Pending,
}

pub struct AuthorizationResponse {
    pub callback: url::Url,
    pub response: url::Url,
}

pub struct AuthorizationFailure {
    pub error: Box<Error>,
    pub callback: Option<url::Url>,
}

impl From<Error> for AuthorizationFailure {
    fn from(error: Error) -> Self {
        Self {
            error: Box::new(error),
            callback: None,
        }
    }
}

impl From<Box<Error>> for AuthorizationFailure {
    fn from(error: Box<Error>) -> Self {
        Self {
            error,
            callback: None,
        }
    }
}

pub async fn prepare<C: ConnectionTrait>(
    db: &C,
    input: &AuthorizationInput,
) -> Result<Prepared, AuthorizationFailure> {
    let application = oauth_application::Entity::find()
        .filter(
            oauth_application::Column::ClientId.eq(input.value("client_id").unwrap_or_default()),
        )
        .lock_shared()
        .one(db)
        .await
        .map_err(|_| Error::PrimitiveError)?
        .ok_or(Error::Ignore)?;
    let mut endpoint = endpoint(application.clone(), None, input)?;
    let pending = match authorization::authorization_code(&mut endpoint, input).await {
        Ok(pending) => pending,
        Err(error) => {
            let callback = if matches!(error, Error::Redirect(_)) {
                let redirect: Option<ExactUrl> = input
                    .value("redirect_uri")
                    .map(str::parse)
                    .transpose()
                    .map_err(|_| Error::Ignore)?;
                let bound = endpoint
                    .registrar
                    .bound_redirect(ClientUrl {
                        client_id: input.value("client_id").unwrap_or_default().into(),
                        redirect_uri: redirect.map(Cow::Owned),
                    })
                    .await
                    .map_err(|_| Error::Ignore)?;
                Some(bound.redirect_uri.to_url())
            } else {
                None
            };
            return Err(AuthorizationFailure {
                error: Box::new(error),
                callback,
            });
        }
    };
    Ok(Prepared {
        application,
        pending,
    })
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HouseholdConsentAuthority {
    household_id: crate::models::ids::HouseholdId,
    membership_id: crate::models::ids::HouseholdMembershipId,
    person_id: crate::models::ids::PersonId,
    permissions_version: i32,
}

pub enum ConsentContext {
    Native,
    Household {
        authority: HouseholdConsentAuthority,
        household_name: String,
        person_name: String,
    },
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
enum ConsentAuthority {
    Native,
    Household(HouseholdConsentAuthority),
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsentBinding {
    account_id: crate::models::ids::AccountId,
    session_reference: String,
    authority: ConsentAuthority,
}

impl ConsentBinding {
    fn new(principal: &BrowserPrincipal, context: &ConsentContext) -> Self {
        let authority = match context {
            ConsentContext::Native => ConsentAuthority::Native,
            ConsentContext::Household { authority, .. } => {
                ConsentAuthority::Household(authority.clone())
            }
        };
        Self {
            account_id: crate::models::ids::AccountId::from_database(principal.account_id()),
            session_reference: principal.provenance().reference.clone(),
            authority,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ConsentApproval {
    pub id: uuid::Uuid,
    binding: ConsentBinding,
    expires_at: chrono::DateTime<chrono::Utc>,
}

fn database_unavailable(error: sea_orm::DbErr) -> ExchangeError {
    let _ = crate::models::errors::OperationError::from(error);
    ExchangeError::Unavailable
}

pub async fn context(
    db: &DatabaseConnection,
    principal: &BrowserPrincipal,
    mobile: bool,
) -> Result<(ConsentContext, ConsentApproval), ExchangeError> {
    let (transaction, _) = principal
        .authorization_transaction(db)
        .await
        .map_err(|_| ExchangeError::Unavailable)?;
    let context = owner_context(&transaction, principal, mobile).await?;
    let clock = transaction
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT clock_timestamp()+interval '5 minutes' AS expires_at",
        ))
        .await
        .map_err(database_unavailable)?
        .ok_or(ExchangeError::Unavailable)?;
    let approval = ConsentApproval {
        id: uuid::Uuid::new_v4(),
        binding: ConsentBinding::new(principal, &context),
        expires_at: clock
            .try_get("", "expires_at")
            .map_err(database_unavailable)?,
    };
    transaction.commit().await.map_err(database_unavailable)?;
    Ok((context, approval))
}

async fn owner_context(
    transaction: &DatabaseTransaction,
    principal: &BrowserPrincipal,
    mobile: bool,
) -> Result<ConsentContext, ExchangeError> {
    if mobile {
        return Ok(ConsentContext::Native);
    }
    let account_id = crate::models::ids::AccountId::from_database(principal.account_id());
    let selected = transaction.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT active_household_id FROM public.identity_sessions WHERE account_id=$1 AND token=$2",
        [account_id.get().into(),principal.provenance().reference.clone().into()]))
        .await.map_err(database_unavailable)?.ok_or(ExchangeError::InvalidGrant)?;
    let household_id = selected
        .try_get::<Option<i64>>("", "active_household_id")
        .map_err(database_unavailable)?
        .map(crate::models::ids::HouseholdId::from_database)
        .ok_or(ExchangeError::InvalidGrant)?;
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT set_config('med_tracker.current_household_id',$1,true)",
            [household_id.to_string().into()],
        ))
        .await
        .map_err(database_unavailable)?;
    let current = transaction
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT med_tracker.current_household_id() AS id",
        ))
        .await
        .map_err(database_unavailable)?
        .ok_or(ExchangeError::Unavailable)?
        .try_get::<Option<i64>>("", "id")
        .map_err(database_unavailable)?;
    if current != Some(household_id.get()) {
        return Err(ExchangeError::InvalidGrant);
    }
    transaction
        .execute_unprepared("SELECT med_tracker.lock_current_household()")
        .await
        .map_err(database_unavailable)?;
    let row = transaction.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT m.id,m.person_id,m.permissions_version,h.name AS household_name,a.name AS person_name FROM public.accounts a JOIN public.household_memberships m ON m.person_id=a.person_id JOIN public.households h ON h.id=m.household_id WHERE a.id=$1 AND h.id=$2 AND m.status='active' AND m.revoked_at IS NULL AND h.status='active' AND h.lifecycle_state='active'",
        [account_id.get().into(),household_id.get().into()])).await.map_err(database_unavailable)?.ok_or(ExchangeError::InvalidGrant)?;
    Ok(ConsentContext::Household {
        authority: HouseholdConsentAuthority {
            household_id,
            membership_id: crate::models::ids::HouseholdMembershipId::from_database(
                row.try_get("", "id").map_err(database_unavailable)?,
            ),
            person_id: crate::models::ids::PersonId::from_database(
                row.try_get("", "person_id").map_err(database_unavailable)?,
            ),
            permissions_version: row
                .try_get("", "permissions_version")
                .map_err(database_unavailable)?,
        },
        household_name: row
            .try_get("", "household_name")
            .map_err(database_unavailable)?,
        person_name: row
            .try_get("", "person_name")
            .map_err(database_unavailable)?,
    })
}

pub async fn approve(
    db: &DatabaseConnection,
    principal: &BrowserPrincipal,
    input: &AuthorizationInput,
    approval: &ConsentApproval,
    request_id: Option<&str>,
) -> Result<AuthorizationResponse, AuthorizationFailure> {
    let (transaction, authenticated_at) = principal
        .authorization_transaction(db)
        .await
        .map_err(|_| Error::Ignore)?;
    let prepared = prepare(&transaction, input).await?;
    let callback = prepared.pending.pre_grant().redirect_uri.to_url();
    let context = owner_context(
        &transaction,
        principal,
        prepared.application.client_kind == "mobile",
    )
    .await
    .map_err(|error| {
        if matches!(error, ExchangeError::Unavailable) {
            Error::PrimitiveError
        } else {
            Error::Ignore
        }
    })?;
    if ConsentBinding::new(principal, &context) != approval.binding {
        return Err(Error::Ignore.into());
    }
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 73015))",
            [approval.id.to_string().into()],
        ))
        .await
        .map_err(|error| {
            database_unavailable(error);
            Error::PrimitiveError
        })?;
    let available = transaction.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT clock_timestamp() < $1 AND NOT EXISTS(SELECT 1 FROM oauth_grants WHERE consent_request_id=$2) AS available",
        [approval.expires_at.into(),approval.id.into()]))
        .await.map_err(|error| { database_unavailable(error); Error::PrimitiveError })?
        .ok_or(Error::PrimitiveError)?;
    if !available
        .try_get::<bool>("", "available")
        .map_err(|error| {
            database_unavailable(error);
            Error::PrimitiveError
        })?
    {
        return Err(Error::Ignore.into());
    }
    let writer = Writer {
        transaction: &transaction,
        application: prepared.application.clone(),
        input,
        account_id: crate::models::ids::AccountId::from_database(principal.account_id()),
        authenticated_at,
        consent_request_id: approval.id,
        context,
        request_id,
    };
    let mut endpoint = endpoint(prepared.application, Some(writer), input)?;
    let url = prepared
        .pending
        .authorize(&mut endpoint, principal.account_id().to_string().into())
        .await?;
    transaction.commit().await.map_err(|error| {
        database_unavailable(error);
        Error::PrimitiveError
    })?;
    Ok(AuthorizationResponse {
        callback,
        response: url,
    })
}

pub async fn deny(
    db: &DatabaseConnection,
    input: &AuthorizationInput,
) -> Result<AuthorizationResponse, AuthorizationFailure> {
    let prepared = prepare(db, input).await?;
    let callback = prepared.pending.pre_grant().redirect_uri.to_url();
    match prepared.pending.deny() {
        Err(Error::Redirect(error)) => Ok(AuthorizationResponse {
            callback,
            response: error.into(),
        }),
        _ => Err(Error::PrimitiveError.into()),
    }
}

struct Endpoint<'a> {
    registrar: RegisteredClient,
    addons: AddonList,
    authorizer: GrantWriter<'a>,
}
struct GrantWriter<'a>(Option<Writer<'a>>);
struct Writer<'a> {
    transaction: &'a DatabaseTransaction,
    application: oauth_application::Model,
    input: &'a AuthorizationInput,
    account_id: crate::models::ids::AccountId,
    authenticated_at: chrono::NaiveDateTime,
    consent_request_id: uuid::Uuid,
    context: ConsentContext,
    request_id: Option<&'a str>,
}

fn endpoint<'a>(
    application: oauth_application::Model,
    writer: Option<Writer<'a>>,
    _: &'a AuthorizationInput,
) -> Result<Endpoint<'a>, Box<Error>> {
    let mut addons = AddonList::new();
    addons.push_authorization(store::pkce(application.client_kind == "mobile"));
    Ok(Endpoint {
        registrar: RegisteredClient::authorization(application)
            .map_err(|_| Error::PrimitiveError)?,
        addons,
        authorizer: GrantWriter(writer),
    })
}

impl authorization::Endpoint for Endpoint<'_> {
    fn registrar(&self) -> &(dyn oxide_auth_async::primitives::Registrar + Sync) {
        &self.registrar
    }
    fn authorizer(&mut self) -> &mut (dyn Authorizer + Send) {
        &mut self.authorizer
    }
    fn extension(&mut self) -> &mut (dyn authorization::Extension + Send) {
        &mut self.addons
    }
}

#[async_trait]
impl Authorizer for GrantWriter<'_> {
    async fn authorize(&mut self, grant: Grant) -> Result<String, ()> {
        let writer = self.0.as_ref().ok_or(())?;
        if grant.owner_id != writer.account_id.to_string()
            || grant.client_id != writer.application.client_id
        {
            return Err(());
        }
        let mut generator = RandomGenerator::new(32);
        let code =
            URL_SAFE_NO_PAD.encode(STANDARD.decode(generator.tag(0, &grant)?).map_err(|_| ())?);
        let (membership_id, person_id, permissions_version) = match &writer.context {
            ConsentContext::Native => (None, None, None),
            ConsentContext::Household { authority, .. } => (
                Some(authority.membership_id.get()),
                Some(authority.person_id.get()),
                Some(authority.permissions_version),
            ),
        };
        let inserted = writer.transaction.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO oauth_grants (account_id,oauth_application_id,client_kind,household_membership_id,person_id,permissions_version,code,code_challenge,code_challenge_method,redirect_uri,expires_in,scopes,access_type,authenticated_at,last_used_at,created_at,updated_at,device_name,consent_request_id) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,LEAST($11,timezone('UTC',clock_timestamp())+interval '5 minutes'),$12,'offline',$13,timezone('UTC',clock_timestamp()),timezone('UTC',clock_timestamp()),timezone('UTC',clock_timestamp()),$14,$15) RETURNING id",
            [writer.account_id.get().into(),writer.application.id.into(),writer.application.client_kind.clone().into(),membership_id.into(),person_id.into(),permissions_version.into(),code.clone().into(),writer.input.value("code_challenge").map(str::to_owned).into(),writer.input.value("code_challenge_method").map(str::to_owned).into(),grant.redirect_uri.to_string().into(),grant.until.naive_utc().into(),grant.scope.to_string().into(),writer.authenticated_at.into(),writer.application.name.clone().into(),writer.consent_request_id.into()])).await.map_err(|error| { database_unavailable(error); })?.ok_or(())?;
        let id: i64 = inserted.try_get("", "id").map_err(|error| {
            database_unavailable(error);
        })?;
        let stored = oauth_grant::Entity::find_by_id(id)
            .one(writer.transaction)
            .await
            .map_err(|error| {
                database_unavailable(error);
            })?
            .ok_or(())?;
        super::audit::record(
            writer.transaction,
            &stored,
            "consent_granted",
            writer.request_id,
        )
        .await
        .map_err(|_| ())?;
        Ok(code)
    }
    async fn extract(&mut self, _: &str) -> Result<Option<Grant>, ()> {
        Err(())
    }
}
