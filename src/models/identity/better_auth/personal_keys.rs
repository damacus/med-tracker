use super::{
    ClinicalAuthSchema, ClinicalStore, IdentityService,
    store::{clinical_id, context, database_error, statement},
};
use crate::models::{
    access::{self, Actor, HouseholdScope, TenantTransaction},
    identity::resource::AuthenticationError,
};
use async_trait::async_trait;
use better_auth::plugins::{
    ApiKeyPlugin,
    api_key::{CreateKeyRequest, RateLimitDefaults, VerifyApiKey},
};
use better_auth_core::{
    AuthContext, AuthError, AuthPlugin, AuthRequest, AuthResponse, AuthResult, AuthRoute,
    HttpMethod, UpdateApiKey, wire::UserView,
};
use sea_orm::ConnectionTrait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};

pub(super) const CONFIG: &str = "medtracker-personal";
const PERMISSIONS: [&str; 5] = [
    "care:read",
    "care:write",
    "doses:write",
    "members:write",
    "invitations:write",
];

#[derive(Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Creation {
    pub name: String,
    pub households: Vec<i64>,
    pub permissions: Vec<String>,
    pub expires_days: u16,
}

fn plugin() -> ApiKeyPlugin {
    ApiKeyPlugin::builder()
        .config_id(CONFIG.to_owned())
        .key_length(64)
        .prefix("medtracker_".to_owned())
        .enable_metadata(true)
        .store_starting_characters(false)
        .rate_limit(RateLimitDefaults {
            enabled: true,
            time_window: 60_000.0,
            max_requests: 120.0,
        })
        .build()
}

pub(super) async fn validate(
    store: &ClinicalStore,
    user: &UserView,
    input: &Creation,
) -> AuthResult<()> {
    if input.name.trim().is_empty()
        || input.name.len() > 100
        || !(1..=365).contains(&input.expires_days)
        || input.households.is_empty()
        || input.households.len() > 20
        || input.households.iter().any(|id| *id <= 0)
        || input.permissions.is_empty()
        || input
            .permissions
            .iter()
            .any(|permission| !PERMISSIONS.contains(&permission.as_str()))
        || input.households.iter().collect::<BTreeSet<_>>().len() != input.households.len()
        || input.permissions.iter().collect::<BTreeSet<_>>().len() != input.permissions.len()
    {
        return Err(AuthError::bad_request(
            "Choose a name, households, explicit permissions and an expiry of 1 to 365 days",
        ));
    }
    let transaction = store.account_transaction().await?;
    let mut households = input.households.clone();
    households.sort_unstable();
    for household in households {
        context(
            &transaction,
            "med_tracker.current_household_id",
            &household.to_string(),
        )
        .await?;
        transaction.query_one_raw(statement("SELECT id FROM public.households WHERE id=$1 AND status='active' AND lifecycle_state='active' FOR SHARE", [household.into()])).await.map_err(database_error)?.ok_or(AuthError::InvalidCredentials)?;
        transaction.query_one_raw(statement("SELECT id FROM public.household_memberships WHERE account_id=$1 AND household_id=$2 AND status='active' AND revoked_at IS NULL FOR SHARE", [clinical_id(&user.id)?.into(), household.into()])).await.map_err(database_error)?.ok_or(AuthError::InvalidCredentials)?;
        let (membership, _) = access::verify_membership(
            &transaction,
            &HouseholdScope {
                actor: Actor {
                    account_id: clinical_id(&user.id)?,
                },
                household_id: household,
                request_id: super::request::request_id().unwrap_or_default(),
            },
        )
        .await
        .map_err(|_| AuthError::forbidden("Household access denied"))?;
        if input
            .permissions
            .iter()
            .any(|permission| matches!(permission.as_str(), "members:write" | "invitations:write"))
            && !crate::models::authorization::household_manager(&membership, household)
        {
            return Err(AuthError::forbidden(
                "Household administration access is required",
            ));
        }
    }
    transaction.commit().await.map_err(database_error)
}

pub(super) async fn create(
    store: &ClinicalStore,
    ctx: &AuthContext<ClinicalAuthSchema>,
    user: &UserView,
    input: &Creation,
) -> AuthResult<AuthResponse> {
    validate(store, user, input).await?;
    let issued = plugin()
        .create_key(
            ctx,
            &CreateKeyRequest {
                config_id: Some(CONFIG.into()),
                user_id: Some(user.id.clone()),
                name: Some(input.name.trim().into()),
                expires_in: Some(f64::from(input.expires_days) * 86_400.0),
                permissions: Some(HashMap::from([(
                    "medtracker".into(),
                    input.permissions.clone(),
                )])),
                metadata: Some(json!({"households":input.households})),
                ..Default::default()
            },
        )
        .await?;
    super::mail::notice(store, user, "API key created", "A personal API key was created for your MedTracker account. Review its access in account security.").await?;
    Ok(AuthResponse::json(
        200,
        &json!({"status":true,"apiKey":issued.key,"name":input.name,"expiresAt":issued.api_key.expires_at}),
    )?)
}

pub async fn listing(
    service: &IdentityService,
    request: &AuthRequest,
    request_id: String,
) -> AuthResult<Value> {
    super::request::trusted(service, request, request_id, async {
        let (user, session) = service.context().require_authoritative_session(request).await?;
        let transaction = service.store.transaction().await?;
        service.store.lock_security_session(&transaction, &session).await?;
        let keys = service.context().database.list_api_keys_by_reference(&user.id).await?.into_iter().filter(|key| key.config_id == CONFIG && key.enabled).map(|key| json!({"id":key.id,"name":key.name,"expires":key.expires_at,"last_used":key.last_request,"permissions":key.permissions})).collect::<Vec<_>>();
        let households = service.context().database.list_user_organizations(&user.id).await?;
        transaction.commit().await.map_err(database_error)?;
        Ok(json!({"keys":keys,"households":households}))
    }).await
}

pub(super) struct PersonalKeys(pub Arc<ClinicalStore>);

#[async_trait]
impl AuthPlugin<ClinicalAuthSchema> for PersonalKeys {
    fn name(&self) -> &'static str {
        "clinical-personal-api-keys"
    }
    fn routes(&self) -> Vec<AuthRoute> {
        vec![AuthRoute::post(
            "/security/api-key/revoke",
            "revoke_personal_api_key",
        )]
    }
    async fn on_request(
        &self,
        request: &AuthRequest,
        ctx: &AuthContext<ClinicalAuthSchema>,
    ) -> AuthResult<Option<AuthResponse>> {
        if request.method != HttpMethod::Post || request.path != "/security/api-key/revoke" {
            return Ok(None);
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Revoke {
            key_id: String,
        }
        let input: Revoke = request
            .body_as_json()
            .map_err(|_| AuthError::bad_request("Invalid key revocation"))?;
        let (user, session) = ctx.require_authoritative_session(request).await?;
        let transaction = self.0.transaction().await?;
        self.0.lock_security_session(&transaction, &session).await?;
        let key = ctx
            .database
            .get_api_key_by_id(&input.key_id)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;
        if key.reference_id != user.id || key.config_id != CONFIG {
            return Err(AuthError::InvalidCredentials);
        }
        ctx.database
            .update_api_key(
                &key.id,
                UpdateApiKey {
                    enabled: Some(false),
                    ..Default::default()
                },
            )
            .await?;
        super::mail::notice(
            &self.0,
            &user,
            "API key revoked",
            "A personal API key was revoked from your MedTracker account.",
        )
        .await?;
        transaction.commit().await.map_err(database_error)?;
        Ok(Some(AuthResponse::json(200, &json!({"status":true}))?))
    }
}

pub(crate) struct Principal {
    pub id: String,
    pub account_id: i64,
    pub permission: String,
    pub time_zone: chrono_tz::Tz,
}

pub(crate) async fn authenticate(
    service: &IdentityService,
    secret: &str,
    permission: &str,
) -> Result<Principal, AuthenticationError> {
    let permissions = permission.split(',').collect::<Vec<_>>();
    if !secret.starts_with("medtracker_")
        || permissions.is_empty()
        || permissions.iter().any(|value| !PERMISSIONS.contains(value))
    {
        return Err(AuthenticationError::Unauthenticated);
    }
    let required = json!({"medtracker":permissions});
    let key = plugin()
        .verify_api_key(
            &VerifyApiKey {
                key: secret,
                config_id: Some(CONFIG),
                permissions: Some(&required),
            },
            service.context(),
        )
        .await
        .map_err(|error| match error {
            better_auth::plugins::api_key::ApiKeyVerificationError::Validation(error)
                if matches!(
                    error.code,
                    better_auth::plugins::api_key::ApiKeyErrorCode::KeyNotFound
                ) =>
            {
                AuthenticationError::Forbidden
            }
            _ => AuthenticationError::Unauthenticated,
        })?;
    let account_id =
        clinical_id(&key.reference_id).map_err(|_| AuthenticationError::Unauthenticated)?;
    let transaction = service
        .store
        .transaction()
        .await
        .map_err(|_| AuthenticationError::Unavailable)?;
    context(
        &transaction,
        "med_tracker.current_account_id",
        &key.reference_id,
    )
    .await
    .map_err(|_| AuthenticationError::Unavailable)?;
    let row = transaction
        .query_one_raw(statement(
            "SELECT preferences FROM public.accounts WHERE id=$1 AND status=2",
            [account_id.into()],
        ))
        .await
        .map_err(|_| AuthenticationError::Unavailable)?
        .ok_or(AuthenticationError::Unauthenticated)?;
    let preferences: Value = row
        .try_get("", "preferences")
        .map_err(|_| AuthenticationError::Unavailable)?;
    let time_zone = super::super::time_zone::preferred(&preferences)?;
    transaction
        .commit()
        .await
        .map_err(|_| AuthenticationError::Unavailable)?;
    Ok(Principal {
        id: key.id,
        account_id,
        permission: permission.into(),
        time_zone,
    })
}

impl Principal {
    pub(crate) async fn revalidate(
        &self,
        tenant: &TenantTransaction,
    ) -> Result<(), AuthenticationError> {
        let transaction = tenant.transaction();
        let scope = tenant.scope();
        transaction
            .query_one_raw(statement(
                "SELECT id FROM public.accounts WHERE id=$1 AND status=2 FOR SHARE",
                [self.account_id.into()],
            ))
            .await
            .map_err(|_| AuthenticationError::Unavailable)?
            .ok_or(AuthenticationError::Unauthenticated)?;
        transaction.query_one_raw(statement("SELECT id FROM public.household_memberships WHERE account_id=$1 AND household_id=$2 AND status='active' AND revoked_at IS NULL FOR SHARE", [self.account_id.into(), scope.household_id.into()])).await.map_err(|_| AuthenticationError::Unavailable)?.ok_or(AuthenticationError::Forbidden)?;
        let row = transaction.query_one_raw(statement("SELECT payload FROM public.identity_api_keys WHERE id=$1 AND account_id=$2 AND expires_at>clock_timestamp() AND payload->>'configId'=$3 AND payload->>'enabled'='true' FOR SHARE", [self.id.clone().into(), self.account_id.into(), CONFIG.into()])).await.map_err(|_| AuthenticationError::Unavailable)?.ok_or(AuthenticationError::Unauthenticated)?;
        let payload: Value = row
            .try_get("", "payload")
            .map_err(|_| AuthenticationError::Unavailable)?;
        let key: better_auth_core::ApiKey =
            serde_json::from_value(payload).map_err(|_| AuthenticationError::Unavailable)?;
        let metadata: Value = serde_json::from_str(
            key.metadata
                .as_deref()
                .ok_or(AuthenticationError::Forbidden)?,
        )
        .map_err(|_| AuthenticationError::Forbidden)?;
        if !metadata["households"].as_array().is_some_and(|values| {
            values
                .iter()
                .any(|value| value.as_i64() == Some(scope.household_id))
        }) {
            return Err(AuthenticationError::Forbidden);
        }
        let permissions: Value = serde_json::from_str(
            key.permissions
                .as_deref()
                .ok_or(AuthenticationError::Forbidden)?,
        )
        .map_err(|_| AuthenticationError::Forbidden)?;
        if !permissions["medtracker"].as_array().is_some_and(|values| {
            self.permission
                .split(',')
                .all(|required| values.iter().any(|value| value.as_str() == Some(required)))
        }) {
            return Err(AuthenticationError::Forbidden);
        }
        access::recheck(tenant)
            .await
            .map_err(super::super::resource::operation_error)
    }
}

pub(crate) fn permission(method: &axum::http::Method, route: &str) -> Option<&'static str> {
    if !route.starts_with("/api/v1/households/") {
        return None;
    }
    if route.ends_with("/push_subscription") {
        return None;
    }
    if route.contains("/admin/invitations") {
        return Some("invitations:write");
    }
    if route.contains("/admin/") {
        return Some("members:write");
    }
    if matches!(*method, axum::http::Method::GET | axum::http::Method::HEAD) {
        return Some("care:read");
    }
    if route.ends_with("/sync/batches") {
        return Some("care:write,doses:write");
    }
    if route.contains("dose_occurrences") || route.ends_with("/medication_takes") {
        return Some("doses:write");
    }
    Some("care:write")
}
