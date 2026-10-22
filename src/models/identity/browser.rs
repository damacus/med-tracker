use crate::models::identity::store::digest as token_digest;
use axum_session::{Session, SessionConfig, SessionMode, SessionStore};
use axum_session_sqlx::SessionPgPool;
use base64::{Engine, engine::general_purpose::STANDARD};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, EntityTrait,
    QueryFilter, QuerySelect, Statement, TransactionTrait, sea_query::Expr,
};
use serde::{Deserialize, Serialize};

mod audit;

use super::{resource::AuthenticationError, store::Lifetime};
use crate::models::{
    access::{self, Actor, HouseholdScope, TenantTransaction},
    care::doses::{CredentialMethod, CredentialProvenance},
    entities::{account, household, identity_session, membership},
    ids::HouseholdId,
    settings,
};

#[derive(Clone, Serialize, Deserialize)]
struct Identity {
    account_id: i64,
    registry_key: String,
    #[serde(default)]
    additional_factor_verified: bool,
    #[serde(default)]
    better_auth: bool,
}

pub struct BrowserPrincipal {
    identity: Identity,
    provenance: CredentialProvenance,
    time_zone: chrono_tz::Tz,
}

#[derive(Serialize)]
pub struct BrowserHousehold {
    pub id: i64,
    pub name: String,
    pub slug: String,
}

pub struct BrowserInvitationActor {
    pub account_id: i64,
    pub person_id: i64,
    pub email: String,
    pub provenance: CredentialProvenance,
}

pub fn household_name(input: &str) -> Option<&str> {
    let name = input.trim();
    (!name.is_empty() && name.chars().count() <= 100 && !name.chars().any(char::is_control))
        .then_some(name)
}

impl BrowserPrincipal {
    pub(crate) async fn guardian_declaration_transaction(
        &self,
        db: &DatabaseConnection,
        write: bool,
    ) -> Result<DatabaseTransaction, AuthenticationError> {
        let database_error = |error: sea_orm::DbErr| super::resource::operation_error(error.into());
        let transaction = db.begin().await.map_err(database_error)?;
        let checked = async {
            transaction.execute_unprepared("SET LOCAL ROLE med_tracker_app; SET LOCAL search_path=pg_catalog,public,pg_temp; SET LOCAL lock_timeout='5s'; SET LOCAL statement_timeout='30s'; SELECT set_config('med_tracker.current_account_id','',true),set_config('med_tracker.current_household_id','',true),set_config('med_tracker.current_membership_id','',true),set_config('med_tracker.current_invitation_token_digest','',true)").await.map_err(database_error)?;
            transaction.execute_raw(sql("SELECT set_config('med_tracker.current_account_id',$1,true)",[self.account_id().to_string().into()])).await.map_err(database_error)?;
            let lock = if write {sea_orm::sea_query::LockType::NoKeyUpdate} else {sea_orm::sea_query::LockType::Share};
            account::Entity::find_by_id(self.account_id()).lock(lock).one(&transaction).await.map_err(database_error)?.ok_or(AuthenticationError::Unauthenticated)?;
            validate(&transaction,&self.identity,false).await?;
            Ok::<_,AuthenticationError>(())
        }.await;
        if let Err(error) = checked {
            transaction.rollback().await.map_err(database_error)?;
            return Err(error);
        }
        Ok(transaction)
    }

    pub async fn revalidate_invitation(
        &self,
        transaction: &DatabaseTransaction,
    ) -> Result<BrowserInvitationActor, AuthenticationError> {
        let account = validate(transaction, &self.identity, false).await?;
        let person = access::person_queries::account_person(Actor {
            account_id: account.id,
        })
        .one(transaction)
        .await
        .map_err(unavailable)?
        .ok_or(AuthenticationError::Unauthenticated)?;
        Ok(BrowserInvitationActor {
            account_id: account.id,
            person_id: person.id,
            email: account.email,
            provenance: self.provenance.clone(),
        })
    }

    pub(super) async fn authorization_transaction(
        &self,
        db: &DatabaseConnection,
    ) -> Result<(DatabaseTransaction, chrono::NaiveDateTime), AuthenticationError> {
        let transaction = transaction(db).await?;
        validate(&transaction, &self.identity, false).await?;
        let query = "SELECT created_at AT TIME ZONE 'UTC' AS created_at FROM public.identity_sessions WHERE account_id=$1 AND token=$2";
        let row = transaction
            .query_one_raw(sql(
                query,
                [
                    self.account_id().into(),
                    self.identity.registry_key.clone().into(),
                ],
            ))
            .await
            .map_err(unavailable)?
            .ok_or(AuthenticationError::Unauthenticated)?;
        let authenticated_at = row.try_get("", "created_at").map_err(unavailable)?;
        Ok((transaction, authenticated_at))
    }
    pub fn account_id(&self) -> i64 {
        self.identity.account_id
    }
    pub(crate) fn session_token(&self) -> &str {
        &self.identity.registry_key
    }
    pub fn provenance(&self) -> &CredentialProvenance {
        &self.provenance
    }
    pub fn time_zone(&self) -> chrono_tz::Tz {
        self.time_zone
    }

    pub async fn create_first_household(
        &self,
        db: &DatabaseConnection,
        name: &str,
        request_id: &str,
        settings: Option<&serde_json::Value>,
    ) -> Result<BrowserHousehold, AuthenticationError> {
        let name = household_name(name).ok_or(AuthenticationError::Forbidden)?;
        let database_error = |error: sea_orm::DbErr| super::resource::operation_error(error.into());
        let transaction = transaction(db).await?;
        account::Entity::find_by_id(self.account_id())
            .lock(sea_orm::sea_query::LockType::NoKeyUpdate)
            .one(&transaction)
            .await
            .map_err(database_error)?
            .ok_or(AuthenticationError::Unauthenticated)?;
        let account = validate(&transaction, &self.identity, false).await?;
        let person = access::person_queries::account_person(Actor {
            account_id: account.id,
        })
        .one(&transaction)
        .await
        .map_err(database_error)?
        .ok_or(AuthenticationError::Unauthenticated)?;
        if person.person_type == 1 {
            return Err(AuthenticationError::Forbidden);
        }
        let time_zone = super::time_zone::preferred(&account.preferences, settings)?;
        let existing = transaction
            .query_one_raw(sql(
                "SELECT 1 FROM household_memberships m JOIN households h ON h.id=m.household_id WHERE m.person_id=$1 AND m.status='active' AND m.revoked_at IS NULL AND h.status='active' AND h.lifecycle_state='active' LIMIT 1",
                [person.id.into()],
            ))
            .await
            .map_err(database_error)?;
        if existing.is_some() {
            return Err(AuthenticationError::Forbidden);
        }
        let mut suffix = [0_u8; 4];
        getrandom::fill(&mut suffix).map_err(unavailable)?;
        let slug = format!("{}-{}", slug::slugify(name), hex::encode(suffix));
        let row = transaction
            .query_one_raw(sql(
                "INSERT INTO households(name,slug,timezone,created_by_account_id,created_at,updated_at) VALUES($1,$2,$3,$4,now(),now()) RETURNING id",
                [name.into(), slug.clone().into(), time_zone.to_string().into(), account.id.into()],
            ))
            .await
            .map_err(database_error)?
            .ok_or(AuthenticationError::Unavailable)?;
        let household_id: i64 = row.try_get("", "id").map_err(database_error)?;
        transaction
            .execute_raw(sql(
                "SELECT set_config('med_tracker.current_household_id',$1,true)",
                [household_id.to_string().into()],
            ))
            .await
            .map_err(database_error)?;
        let row = transaction
            .query_one_raw(sql(
                "INSERT INTO household_memberships(household_id,person_id,role,status,permissions_version,joined_at,created_at,updated_at) VALUES($1,$2,'owner','active',1,now(),now(),now()) RETURNING id",
                [household_id.into(), person.id.into()],
            ))
            .await
            .map_err(database_error)?
            .ok_or(AuthenticationError::Unavailable)?;
        let membership_id: i64 = row.try_get("", "id").map_err(database_error)?;
        let metadata = serde_json::json!({"target_account_id":account.id,"target_membership_id":membership_id,"previous_state":null,"new_state":{"role":"owner","status":"active","person_id":person.id,"permissions_version":1},"outcome":"success"});
        let audit_context = serde_json::json!({"actor_account_id":account.id,"actor_person_id":person.id,"actor_membership_id":membership_id,"household_id":household_id,"request_id":request_id});
        transaction
            .execute_raw(sql(
                "INSERT INTO security_audit_events(household_id,actor_account_id,actor_membership_id,event_type,metadata,audit_context,request_id,created_at,updated_at) VALUES($1,$2,$3,'household_access.membership_created',$4,$5,$6,now(),now())",
                [household_id.into(),account.id.into(),membership_id.into(),metadata.into(),audit_context.into(),request_id.to_owned().into()],
            ))
            .await
            .map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(BrowserHousehold {
            id: household_id,
            name: name.to_owned(),
            slug,
        })
    }

    pub async fn households(
        &self,
        db: &DatabaseConnection,
    ) -> Result<Vec<BrowserHousehold>, AuthenticationError> {
        let transaction = transaction(db).await?;
        validate(&transaction, &self.identity, false).await?;
        let rows = transaction.query_all_raw(sql(
            "SELECT h.id, h.name, h.slug FROM households h JOIN household_memberships m ON m.household_id = h.id JOIN accounts a ON a.person_id=m.person_id WHERE a.id = $1 AND m.status = 'active' AND m.revoked_at IS NULL AND h.status = 'active' AND h.lifecycle_state = 'active' ORDER BY h.name, h.id",
            [self.account_id().into()],
        )).await.map_err(unavailable)?;
        let households = rows
            .into_iter()
            .map(|row| {
                Ok(BrowserHousehold {
                    id: row.try_get("", "id").map_err(unavailable)?,
                    name: row.try_get("", "name").map_err(unavailable)?,
                    slug: row.try_get("", "slug").map_err(unavailable)?,
                })
            })
            .collect::<Result<Vec<_>, AuthenticationError>>()?;
        transaction.commit().await.map_err(unavailable)?;
        Ok(households)
    }

    pub(crate) async fn select_household(
        &self,
        db: &DatabaseConnection,
        household_id: HouseholdId,
    ) -> Result<BrowserHousehold, AuthenticationError> {
        let transaction = transaction(db).await?;
        let account = validate(&transaction, &self.identity, false).await?;
        membership::Entity::find()
            .filter(membership::Column::PersonId.eq(account.person_id))
            .filter(membership::Column::HouseholdId.eq(household_id.get()))
            .filter(membership::Column::Status.eq("active"))
            .filter(membership::Column::RevokedAt.is_null())
            .lock(sea_orm::sea_query::LockType::Share)
            .one(&transaction)
            .await
            .map_err(unavailable)?
            .ok_or(AuthenticationError::Forbidden)?;
        let household = household::Entity::find_by_id(household_id.get())
            .filter(household::Column::Status.eq("active"))
            .filter(household::Column::LifecycleState.eq("active"))
            .lock(sea_orm::sea_query::LockType::Share)
            .one(&transaction)
            .await
            .map_err(unavailable)?
            .ok_or(AuthenticationError::Forbidden)?;
        let updated = identity_session::Entity::update_many()
            .col_expr(
                identity_session::Column::ActiveHouseholdId,
                Expr::value(household.id),
            )
            .filter(identity_session::Column::UserId.eq(account.id))
            .filter(identity_session::Column::Token.eq(self.session_token()))
            .filter(identity_session::Column::Active.eq(true))
            .filter(identity_session::Column::ClinicalSessionPurpose.eq("authenticated"))
            .filter(identity_session::Column::ExpiresAt.gt(chrono::Utc::now()))
            .exec(&transaction)
            .await
            .map_err(unavailable)?;
        if updated.rows_affected != 1 {
            return Err(AuthenticationError::Unauthenticated);
        }
        transaction.commit().await.map_err(unavailable)?;
        Ok(BrowserHousehold {
            id: household.id,
            name: household.name,
            slug: household.slug,
        })
    }

    pub async fn active_household(
        &self,
        db: &DatabaseConnection,
    ) -> Result<Option<BrowserHousehold>, AuthenticationError> {
        let transaction = transaction(db).await?;
        validate(&transaction, &self.identity, false).await?;
        let row = transaction.query_one_raw(sql(
            "SELECT h.id,h.name,h.slug FROM public.identity_sessions s JOIN public.households h ON h.id=s.active_household_id JOIN public.household_memberships m ON m.household_id=h.id AND m.person_id=(SELECT a.person_id FROM public.accounts a WHERE a.id=s.account_id) WHERE s.account_id=$1 AND s.token=$2 AND s.active AND s.purpose='authenticated' AND s.expires_at>clock_timestamp() AND h.status='active' AND h.lifecycle_state='active' AND m.status='active' AND m.revoked_at IS NULL",
            [self.account_id().into(), self.session_token().into()],
        )).await.map_err(unavailable)?;
        let household = row
            .map(|row| {
                Ok(BrowserHousehold {
                    id: row.try_get("", "id").map_err(unavailable)?,
                    name: row.try_get("", "name").map_err(unavailable)?,
                    slug: row.try_get("", "slug").map_err(unavailable)?,
                })
            })
            .transpose()?;
        transaction.commit().await.map_err(unavailable)?;
        Ok(household)
    }

    pub async fn begin_household_slug(
        &self,
        db: &DatabaseConnection,
        slug: &str,
        request_id: String,
        settings: Option<&serde_json::Value>,
    ) -> Result<TenantTransaction, AuthenticationError> {
        let lookup = transaction(db).await?;
        validate(&lookup, &self.identity, false).await?;
        let selected = household::Entity::find()
            .filter(household::Column::Slug.eq(slug))
            .one(&lookup)
            .await
            .map_err(unavailable)?
            .ok_or(AuthenticationError::Forbidden)?;
        lookup.commit().await.map_err(unavailable)?;
        let tenant = access::begin(
            db,
            &HouseholdScope {
                actor: Actor {
                    account_id: self.account_id(),
                },
                household_id: selected.id,
                request_id,
            },
            settings,
        )
        .await
        .map_err(super::resource::operation_error)?;
        if let Err(error) = validate(tenant.transaction(), &self.identity, false).await {
            tenant.rollback().await.map_err(unavailable)?;
            return Err(error);
        }
        Ok(tenant)
    }
}

fn unavailable<T>(_: T) -> AuthenticationError {
    AuthenticationError::Unavailable
}

fn sql(query: &str, values: impl IntoIterator<Item = sea_orm::Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, query, values)
}

pub(super) async fn transaction(
    db: &DatabaseConnection,
) -> Result<DatabaseTransaction, AuthenticationError> {
    let transaction = db.begin().await.map_err(unavailable)?;
    transaction.execute_unprepared("SET LOCAL ROLE med_tracker_app; SET LOCAL search_path = pg_catalog, public, pg_temp; SET LOCAL lock_timeout = '5s'; SET LOCAL statement_timeout = '30s'; SELECT set_config('med_tracker.current_account_id', '', true), set_config('med_tracker.current_household_id', '', true), set_config('med_tracker.current_membership_id', '', true), set_config('med_tracker.current_invitation_token_digest', '', true)").await.map_err(unavailable)?;
    Ok(transaction)
}

async fn validate(
    transaction: &DatabaseTransaction,
    identity: &Identity,
    _touch: bool,
) -> Result<account::Model, AuthenticationError> {
    if !identity.better_auth {
        return Err(AuthenticationError::Unauthenticated);
    }
    transaction
        .execute_raw(sql(
            "SELECT set_config('med_tracker.current_account_id',$1,true)",
            [identity.account_id.to_string().into()],
        ))
        .await
        .map_err(unavailable)?;
    let valid = transaction.query_one_raw(sql("SELECT account_id FROM public.identity_sessions WHERE account_id=$1 AND token=$2 AND active AND purpose='authenticated' AND expires_at>clock_timestamp() FOR SHARE", [identity.account_id.into(), identity.registry_key.clone().into()])).await.map_err(unavailable)?;
    if valid.is_none() {
        return Err(AuthenticationError::Unauthenticated);
    }
    access::verify_account(transaction, identity.account_id)
        .await
        .map_err(super::resource::operation_error)?;
    account::Entity::find_by_id(identity.account_id)
        .filter(account::Column::Status.eq(2))
        .one(transaction)
        .await
        .map_err(unavailable)?
        .ok_or(AuthenticationError::Unauthenticated)
}

pub async fn authenticate(
    db: &DatabaseConnection,
    session: &Session<SessionPgPool>,
    settings: Option<&serde_json::Value>,
) -> Result<BrowserPrincipal, AuthenticationError> {
    let identity = session
        .get::<Identity>("identity")
        .ok_or(AuthenticationError::Unauthenticated)?;
    let transaction = transaction(db).await?;
    let account = validate(&transaction, &identity, true).await?;
    let time_zone = super::time_zone::preferred(&account.preferences, settings)?;
    transaction.commit().await.map_err(unavailable)?;
    Ok(BrowserPrincipal {
        provenance: CredentialProvenance {
            method: CredentialMethod::BrowserSession,
            reference: token_digest(&identity.registry_key),
        },
        identity,
        time_zone,
    })
}

pub(super) async fn record_auth_token(
    transaction: &DatabaseTransaction,
    account_id: i64,
    token_type: &str,
    action: &str,
    request_id: Option<&str>,
) -> Result<(), AuthenticationError> {
    audit::record(transaction, account_id, token_type, action, request_id).await
}

#[derive(Clone)]
pub struct BrowserLayers {
    pub(crate) identity_secret: String,
    pub(crate) secure: bool,
    store: SessionStore<SessionPgPool>,
    csrf: axum_csrf::CsrfConfig,
}

pub fn route_layers(
    ctx: &loco_rs::app::AppContext,
    routes: loco_rs::controller::Routes,
) -> loco_rs::controller::Routes {
    let routes = match ctx
        .shared_store
        .get::<super::better_auth::IdentityService>()
    {
        Some(service) => routes
            .layer(axum::middleware::from_fn(identity_bridge))
            .layer(axum::Extension(service)),
        None => routes,
    };
    match ctx.shared_store.get::<BrowserLayers>() {
        Some(layers) => routes
            .layer(axum_session::SessionLayer::new(layers.store))
            .layer(axum_csrf::CsrfLayer::new(layers.csrf))
            .layer(
                tower_http::csrf::CsrfLayer::new().with_rejection_response(|_| {
                    use axum::response::IntoResponse;
                    (
                        axum::http::StatusCode::FORBIDDEN,
                        [(axum::http::header::CACHE_CONTROL, "no-store")],
                    )
                        .into_response()
                }),
            ),
        None => routes,
    }
}

async fn identity_bridge(
    axum::Extension(service): axum::Extension<super::better_auth::IdentityService>,
    session: Session<SessionPgPool>,
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    if let Err(status) = super::better_auth::normalize_cookie_headers(request.headers_mut()) {
        return (status, [(axum::http::header::CACHE_CONTROL, "no-store")]).into_response();
    }
    let mut auth =
        better_auth_core::AuthRequest::new(better_auth_core::HttpMethod::Get, "/get-session");
    auth.headers = request
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.to_string(), value.to_owned()))
        })
        .collect();
    let mut destination = None;
    match super::better_auth::browser_identity(&service, &auth).await {
        Ok(super::better_auth::BrowserIdentity::Authenticated {
            user,
            session: identity,
        }) => {
            let Ok(account_id) = user.id.display_string().and_then(|id| {
                id.parse()
                    .map_err(|_| better_auth_core::AuthError::InvalidCredentials)
            }) else {
                return axum::http::StatusCode::UNAUTHORIZED.into_response();
            };
            let Ok(registry_key) = identity.token.typed().cloned() else {
                return axum::http::StatusCode::UNAUTHORIZED.into_response();
            };
            session.set(
                "identity",
                Identity {
                    account_id,
                    registry_key,
                    additional_factor_verified: true,
                    better_auth: true,
                },
            );
            session.set_store(true);
        }
        Ok(super::better_auth::BrowserIdentity::Enrolment { .. }) => {
            session.remove("identity");
            if request.uri().path() == "/" {
                destination = Some("/auth/passkey/setup");
            }
        }
        Ok(super::better_auth::BrowserIdentity::SignedOut) => {
            session.remove("identity");
        }
        Err(_) => return axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
    let mut response = if let Some(destination) = destination {
        axum::response::Redirect::to(destination).into_response()
    } else {
        next.run(request).await
    };
    let mut finalized = better_auth_core::AuthResponse::native(
        response.status().as_u16(),
        better_auth_core::FieldValue::Undefined,
    );
    for cookie in response.headers().get_all(axum::http::header::SET_COOKIE) {
        let Ok(value) = cookie.to_str() else {
            return axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response();
        };
        finalized.headers.append("set-cookie", value.to_owned());
    }
    if service
        .context()
        .session_manager()
        .finish_response(&auth, &mut finalized)
        .is_err()
    {
        return axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    response
        .headers_mut()
        .remove(axum::http::header::SET_COOKIE);
    for cookie in finalized.headers.get_all("set-cookie") {
        let Ok(value) = cookie.parse() else {
            return axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response();
        };
        response
            .headers_mut()
            .append(axum::http::header::SET_COOKIE, value);
    }
    response
}

pub async fn layers(
    ctx: &loco_rs::app::AppContext,
) -> Result<Option<BrowserLayers>, AuthenticationError> {
    let db = &ctx.db;
    let settings = ctx
        .config
        .settings
        .as_ref()
        .map(|settings| &settings["browser_session"]);
    let encoded = match settings.and_then(|settings| settings["key"].as_str()) {
        Some(key) => key.to_owned(),
        None if matches!(ctx.environment, loco_rs::environment::Environment::Test) => {
            return Ok(None);
        }
        None => return Err(AuthenticationError::Unavailable),
    };
    let bytes = STANDARD.decode(&encoded).map_err(unavailable)?;
    if bytes.len() != 64 {
        return Err(AuthenticationError::Unavailable);
    }
    let key = axum_session::Key::from(&bytes);
    let secure = match settings.and_then(|settings| settings.get("secure")) {
        Some(value) => match settings::truthy(value) {
            Some(value) => value,
            None => return Err(AuthenticationError::Unavailable),
        },
        None => true,
    };
    if matches!(
        ctx.environment,
        loco_rs::environment::Environment::Production
    ) && !secure
    {
        return Err(AuthenticationError::Unavailable);
    }
    let policy = Lifetime::from_settings(ctx.config.settings.as_ref()).map_err(unavailable)?;
    db.query_one_raw(Statement::from_string(
        DbBackend::Postgres,
        "SELECT id, expires, session FROM public.browser_sessions LIMIT 0",
    ))
    .await
    .map_err(unavailable)?;
    let config = SessionConfig::default()
        .with_table_name("browser_sessions")
        .with_session_name("medtracker_session")
        .with_key(key.clone())
        .with_database_key(key.clone())
        .with_mode(SessionMode::OptIn)
        .with_lifetime(policy.inactivity)
        .with_max_age(Some(policy.inactivity))
        .with_db_update_interval(chrono::Duration::zero())
        .with_secure(secure)
        .with_http_only(true)
        .with_cookie_same_site(axum_session::SameSite::Lax);
    let mut store = SessionStore::<SessionPgPool>::new(None, config)
        .await
        .map_err(unavailable)?;
    store.client = Some(SessionPgPool::from(
        db.get_postgres_connection_pool().clone(),
    ));
    let csrf = axum_csrf::CsrfConfig::default()
        .with_key(Some(key))
        .with_secure(secure)
        .with_http_only(true)
        .with_cookie_name("medtracker_csrf")
        .with_cookie_same_site(axum_csrf::SameSite::Lax);
    Ok(Some(BrowserLayers {
        identity_secret: encoded,
        secure,
        store,
        csrf,
    }))
}
