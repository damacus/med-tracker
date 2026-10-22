use super::{ExchangeError, browser::BrowserPrincipal, registrar::RegisteredClient, store};
use crate::models::entities::{
    account, household, identity_session, membership, oauth_application, oauth_grant, person,
};
use crate::models::{
    errors::OperationError,
    ids::{AccountId, HouseholdId, MembershipId, PersonId},
};
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

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct AuthorizationInput(pub Vec<(String, String)>);

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct DisplayedConsent {
    pub input: AuthorizationInput,
    pub account_id: AccountId,
    pub context: ConsentContext,
    pub nonce: String,
    session_token_digest: String,
}

impl DisplayedConsent {
    pub fn new(
        input: AuthorizationInput,
        principal: &BrowserPrincipal,
        context: ConsentContext,
        nonce: String,
    ) -> Self {
        Self {
            input,
            account_id: AccountId::from_database(principal.account_id()),
            context,
            nonce,
            session_token_digest: store::digest(principal.session_token()),
        }
    }

    pub fn matches_principal(&self, principal: &BrowserPrincipal) -> bool {
        self.account_id == AccountId::from_database(principal.account_id())
            && self.session_token_digest == store::digest(principal.session_token())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) enum ConsentContext {
    Mobile,
    Household(HouseholdConsentContext),
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct HouseholdConsentContext {
    account_id: AccountId,
    household_id: HouseholdId,
    membership_id: MembershipId,
    person_id: PersonId,
    permissions_version: i32,
    household_name: String,
    person_name: String,
}

fn unavailable(error: sea_orm::DbErr) -> ExchangeError {
    let _ = OperationError::from(error);
    ExchangeError::Unavailable
}

fn authorization_error(error: ExchangeError) -> AuthorizationFailure {
    match error {
        ExchangeError::Unavailable => Error::PrimitiveError.into(),
        _ => Error::Ignore.into(),
    }
}

impl ConsentContext {
    pub fn household_name(&self) -> Option<&str> {
        match self {
            Self::Mobile => None,
            Self::Household(context) => Some(&context.household_name),
        }
    }

    pub fn person_name(&self) -> Option<&str> {
        match self {
            Self::Mobile => None,
            Self::Household(context) => Some(&context.person_name),
        }
    }
}

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
        .map_err(|error| {
            let _ = OperationError::from(error);
            Error::PrimitiveError
        })?
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

pub(crate) async fn context(
    db: &DatabaseConnection,
    principal: &BrowserPrincipal,
    mobile: bool,
) -> Result<Option<ConsentContext>, ExchangeError> {
    let (transaction, _) = principal
        .authorization_transaction(db)
        .await
        .map_err(|error| match error {
            super::resource::AuthenticationError::Unavailable => ExchangeError::Unavailable,
            _ => ExchangeError::InvalidGrant,
        })?;
    let context = if mobile {
        Some(ConsentContext::Mobile)
    } else {
        match selected_household_context(&transaction, principal).await {
            Ok(context) => Some(ConsentContext::Household(context)),
            Err(ExchangeError::InvalidGrant) => None,
            Err(error) => return Err(error),
        }
    };
    transaction.commit().await.map_err(unavailable)?;
    Ok(context)
}

async fn selected_household_context(
    transaction: &DatabaseTransaction,
    principal: &BrowserPrincipal,
) -> Result<HouseholdConsentContext, ExchangeError> {
    let session = identity_session::Entity::find()
        .filter(identity_session::Column::UserId.eq(principal.account_id()))
        .filter(identity_session::Column::Token.eq(principal.session_token()))
        .filter(identity_session::Column::Active.eq(true))
        .filter(identity_session::Column::ClinicalSessionPurpose.eq("authenticated"))
        .filter(identity_session::Column::ExpiresAt.gt(chrono::Utc::now()))
        .lock_shared()
        .one(transaction)
        .await
        .map_err(unavailable)?
        .ok_or(ExchangeError::InvalidGrant)?;
    let household_id = HouseholdId::from_database(
        session
            .active_household_id
            .ok_or(ExchangeError::InvalidGrant)?,
    );
    current_household_context(
        transaction,
        AccountId::from_database(principal.account_id()),
        household_id,
    )
    .await
}

async fn current_household_context(
    transaction: &DatabaseTransaction,
    account_id: AccountId,
    household_id: HouseholdId,
) -> Result<HouseholdConsentContext, ExchangeError> {
    let account = account::Entity::find_by_id(account_id.get())
        .one(transaction)
        .await
        .map_err(unavailable)?
        .ok_or(ExchangeError::InvalidGrant)?;
    let membership = membership::Entity::find()
        .filter(membership::Column::PersonId.eq(account.person_id))
        .filter(membership::Column::HouseholdId.eq(household_id.get()))
        .filter(membership::Column::Status.eq("active"))
        .filter(membership::Column::RevokedAt.is_null())
        .lock_shared()
        .one(transaction)
        .await
        .map_err(unavailable)?
        .ok_or(ExchangeError::InvalidGrant)?;
    let household = household::Entity::find_by_id(household_id.get())
        .filter(household::Column::Status.eq("active"))
        .filter(household::Column::LifecycleState.eq("active"))
        .lock_shared()
        .one(transaction)
        .await
        .map_err(unavailable)?
        .ok_or(ExchangeError::InvalidGrant)?;
    let person = person::Entity::find_by_id(account.person_id)
        .one(transaction)
        .await
        .map_err(unavailable)?
        .ok_or(ExchangeError::InvalidGrant)?;
    Ok(HouseholdConsentContext {
        account_id,
        household_id,
        membership_id: MembershipId::from_database(membership.id),
        person_id: PersonId::from_database(membership.person_id),
        permissions_version: membership.permissions_version,
        household_name: household.name,
        person_name: person.name,
    })
}

pub(crate) async fn approve(
    db: &DatabaseConnection,
    principal: &BrowserPrincipal,
    input: &AuthorizationInput,
    shown: &ConsentContext,
    request_id: Option<&str>,
) -> Result<AuthorizationResponse, AuthorizationFailure> {
    let (transaction, authenticated_at) =
        principal
            .authorization_transaction(db)
            .await
            .map_err(|error| match error {
                super::resource::AuthenticationError::Unavailable => Error::PrimitiveError,
                _ => Error::Ignore,
            })?;
    let prepared = prepare(&transaction, input).await?;
    let callback = prepared.pending.pre_grant().redirect_uri.to_url();
    let context = match (prepared.application.client_kind.as_str(), shown) {
        ("mobile", ConsentContext::Mobile) => ConsentContext::Mobile,
        ("mobile", _) | (_, ConsentContext::Mobile) => return Err(Error::Ignore.into()),
        (_, ConsentContext::Household(shown)) => {
            let current = current_household_context(
                &transaction,
                AccountId::from_database(principal.account_id()),
                shown.household_id,
            )
            .await
            .map_err(authorization_error)?;
            if &current != shown {
                return Err(Error::Ignore.into());
            }
            ConsentContext::Household(current)
        }
    };
    let writer = Writer {
        transaction: &transaction,
        application: prepared.application.clone(),
        input,
        account_id: AccountId::from_database(principal.account_id()),
        authenticated_at,
        context,
        request_id,
    };
    let mut endpoint = endpoint(prepared.application, Some(writer), input)?;
    let url = prepared
        .pending
        .authorize(&mut endpoint, principal.account_id().to_string().into())
        .await?;
    transaction.commit().await.map_err(|error| {
        let _ = OperationError::from(error);
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
    account_id: AccountId,
    authenticated_at: chrono::NaiveDateTime,
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
            ConsentContext::Mobile => (None, None, None),
            ConsentContext::Household(context) => (
                Some(context.membership_id.get()),
                Some(context.person_id.get()),
                Some(context.permissions_version),
            ),
        };
        let inserted = writer.transaction.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO oauth_grants (account_id,oauth_application_id,client_kind,household_membership_id,person_id,permissions_version,code,code_challenge,code_challenge_method,redirect_uri,expires_in,scopes,access_type,authenticated_at,last_used_at,created_at,updated_at,device_name) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,LEAST($11,timezone('UTC',clock_timestamp())+interval '5 minutes'),$12,'offline',$13,timezone('UTC',clock_timestamp()),timezone('UTC',clock_timestamp()),timezone('UTC',clock_timestamp()),$14) RETURNING id",
            [writer.account_id.get().into(),writer.application.id.into(),writer.application.client_kind.clone().into(),membership_id.into(),person_id.into(),permissions_version.into(),code.clone().into(),writer.input.value("code_challenge").map(str::to_owned).into(),writer.input.value("code_challenge_method").map(str::to_owned).into(),grant.redirect_uri.to_string().into(),grant.until.naive_utc().into(),grant.scope.to_string().into(),writer.authenticated_at.into(),writer.application.name.clone().into()])).await.map_err(|_| ())?.ok_or(())?;
        let id: i64 = inserted.try_get("", "id").map_err(|_| ())?;
        let stored = oauth_grant::Entity::find_by_id(id)
            .one(writer.transaction)
            .await
            .map_err(|_| ())?
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
