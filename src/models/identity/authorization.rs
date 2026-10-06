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

pub async fn context(
    db: &DatabaseConnection,
    principal: &BrowserPrincipal,
    mobile: bool,
) -> Result<serde_json::Value, ExchangeError> {
    let (transaction, _) = principal
        .authorization_transaction(db)
        .await
        .map_err(|_| ExchangeError::Unavailable)?;
    let context = owner_context(&transaction, principal.account_id(), mobile).await?;
    transaction
        .commit()
        .await
        .map_err(|_| ExchangeError::Unavailable)?;
    Ok(context)
}

async fn owner_context(
    transaction: &DatabaseTransaction,
    account_id: i64,
    mobile: bool,
) -> Result<serde_json::Value, ExchangeError> {
    if mobile {
        return Ok(serde_json::json!({"household_name":null,"person_name":null}));
    }
    let row = transaction.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT m.id, m.person_id, m.permissions_version, h.name AS household_name, p.name AS person_name FROM household_memberships m JOIN households h ON h.id=m.household_id LEFT JOIN people p ON p.id=m.person_id WHERE m.account_id=$1 AND m.status='active' AND m.revoked_at IS NULL AND h.status='active' AND h.lifecycle_state='active' ORDER BY m.id LIMIT 1 FOR SHARE OF m,h", [account_id.into()])).await.map_err(|_| ExchangeError::Unavailable)?.ok_or(ExchangeError::InvalidGrant)?;
    Ok(
        serde_json::json!({"membership_id":row.try_get::<i64>("","id").map_err(|_| ExchangeError::Unavailable)?,"person_id":row.try_get::<Option<i64>>("","person_id").map_err(|_| ExchangeError::Unavailable)?,"permissions_version":row.try_get::<i32>("","permissions_version").map_err(|_| ExchangeError::Unavailable)?,"household_name":row.try_get::<String>("","household_name").map_err(|_| ExchangeError::Unavailable)?,"person_name":row.try_get::<Option<String>>("","person_name").map_err(|_| ExchangeError::Unavailable)?}),
    )
}

pub async fn approve(
    db: &DatabaseConnection,
    principal: &BrowserPrincipal,
    input: &AuthorizationInput,
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
        principal.account_id(),
        prepared.application.client_kind == "mobile",
    )
    .await
    .map_err(|_| Error::Ignore)?;
    let writer = Writer {
        transaction: &transaction,
        application: prepared.application.clone(),
        input,
        account_id: principal.account_id(),
        authenticated_at,
        context,
        request_id,
    };
    let mut endpoint = endpoint(prepared.application, Some(writer), input)?;
    let url = prepared
        .pending
        .authorize(&mut endpoint, principal.account_id().to_string().into())
        .await?;
    transaction
        .commit()
        .await
        .map_err(|_| Error::PrimitiveError)?;
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
    account_id: i64,
    authenticated_at: chrono::NaiveDateTime,
    context: serde_json::Value,
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
        let inserted = writer.transaction.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO oauth_grants (account_id,oauth_application_id,client_kind,household_membership_id,person_id,permissions_version,code,code_challenge,code_challenge_method,redirect_uri,expires_in,scopes,access_type,authenticated_at,last_used_at,created_at,updated_at,device_name) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,LEAST($11,timezone('UTC',clock_timestamp())+interval '5 minutes'),$12,'offline',$13,timezone('UTC',clock_timestamp()),timezone('UTC',clock_timestamp()),timezone('UTC',clock_timestamp()),$14) RETURNING id",
            [writer.account_id.into(),writer.application.id.into(),writer.application.client_kind.clone().into(),writer.context["membership_id"].as_i64().into(),writer.context["person_id"].as_i64().into(),writer.context["permissions_version"].as_i64().into(),code.clone().into(),writer.input.value("code_challenge").map(str::to_owned).into(),writer.input.value("code_challenge_method").map(str::to_owned).into(),grant.redirect_uri.to_string().into(),grant.until.naive_utc().into(),grant.scope.to_string().into(),writer.authenticated_at.into(),writer.application.name.clone().into()])).await.map_err(|_| ())?.ok_or(())?;
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
