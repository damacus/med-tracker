use super::IdentityService;
use crate::models::ids::AccountId;
use better_auth_core::{AuthRequest, AuthResult, SchemaValue};
use chrono::{DateTime, Utc};
use serde::{Serialize, Serializer};

#[derive(sea_orm::FromQueryResult)]
pub(crate) struct SessionSummary {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(sea_orm::FromQueryResult)]
pub(crate) struct PasskeySummary {
    pub id: i64,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub(crate) struct Inventory {
    pub sessions: Vec<SessionSummary>,
    pub passkeys: Vec<PasskeySummary>,
}

pub(crate) async fn inventory(
    db: &impl sea_orm::ConnectionTrait,
    account_id: i64,
) -> AuthResult<Inventory> {
    use super::schema::{passkey, session};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
    let sessions = session::Entity::find()
        .select_only()
        .columns([
            session::Column::Id,
            session::Column::CreatedAt,
            session::Column::ExpiresAt,
        ])
        .filter(session::Column::UserId.eq(account_id))
        .filter(session::Column::Active.eq(true))
        .filter(session::Column::ExpiresAt.gt(Utc::now()))
        .order_by_asc(session::Column::CreatedAt)
        .order_by_asc(session::Column::Id)
        .into_model::<SessionSummary>()
        .all(db)
        .await
        .map_err(super::policy::database_error)?;
    let passkeys = passkey::Entity::find()
        .select_only()
        .columns([
            passkey::Column::Id,
            passkey::Column::Name,
            passkey::Column::CreatedAt,
        ])
        .filter(passkey::Column::UserId.eq(account_id))
        .order_by_asc(passkey::Column::CreatedAt)
        .order_by_asc(passkey::Column::Id)
        .into_model::<PasskeySummary>()
        .all(db)
        .await
        .map_err(super::policy::database_error)?;
    Ok(Inventory { sessions, passkeys })
}

#[derive(Serialize)]
pub(crate) struct SecuritySessionId(i64);

#[derive(Serialize)]
pub(crate) struct SecurityPasskeyId(i64);

#[derive(Serialize)]
pub(crate) struct SecurityKeyId(String);

pub(crate) struct SecuritySession {
    pub(crate) id: SecuritySessionId,
    pub(crate) current: bool,
    pub(crate) created: DateTime<Utc>,
    pub(crate) expires: DateTime<Utc>,
}

#[derive(Serialize)]
pub(crate) struct SecurityPasskey {
    id: SecurityPasskeyId,
    name: Option<String>,
    #[serde(serialize_with = "display_date")]
    created: DateTime<Utc>,
}

#[derive(Serialize)]
pub(crate) struct SecurityKey {
    id: SecurityKeyId,
    name: Option<String>,
    expires: Option<DateTime<Utc>>,
    last_used: Option<DateTime<Utc>>,
}

pub(crate) struct SecurityOverview {
    pub account_id: AccountId,
    pub(crate) totp: bool,
    pub(crate) password: bool,
    pub(crate) zitadel: bool,
    pub(crate) provider_available: bool,
    pub(crate) can_unlink_provider: bool,
    pub(crate) passkey_count: usize,
    pub(crate) keys: Vec<SecurityKey>,
    pub(crate) passkeys: Vec<SecurityPasskey>,
    pub(crate) sessions: Vec<SecuritySession>,
}

fn display_date<S: Serializer>(value: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_str(&value.format("%d %b %Y"))
}

pub(crate) async fn overview(
    service: &IdentityService,
    request: &AuthRequest,
    request_id: String,
) -> AuthResult<SecurityOverview> {
    super::request::trusted(service, request, request_id, async {
        let (user, current) = service
            .context()
            .require_authoritative_session(request)
            .await?;
        let transaction = service.policy.transaction().await?;
        service
            .policy
            .require_security_session(&transaction, &current)
            .await?;
        let user_id = user.id.display_string()?;
        let account_id = AccountId::from_database(super::clinical_id(&user_id)?);
        let inventory = inventory(&transaction, account_id.get()).await?;
        let accounts = service
            .context()
            .database
            .get_user_accounts(&user_id)
            .await?;
        let keys = super::personal_keys::inventory(&transaction, account_id.get())
            .await?
            .into_iter()
            .map(|key| SecurityKey {
                id: SecurityKeyId(key.id),
                name: key.name,
                expires: key.expires_at,
                last_used: key.last_request,
            })
            .collect();
        let password = accounts.iter().any(|account| {
            account.provider_id == "credential"
                && matches!(account.password, SchemaValue::Typed(Some(_)))
        });
        let zitadel = accounts
            .iter()
            .any(|account| account.provider_id == "zitadel");
        let passkeys = inventory
            .passkeys
            .into_iter()
            .map(|key| SecurityPasskey {
                id: SecurityPasskeyId(key.id),
                name: key.name,
                created: key.created_at,
            })
            .collect::<Vec<_>>();
        let current_id = super::clinical_id(&current.id.display_string()?)?;
        let sessions = inventory
            .sessions
            .into_iter()
            .map(|session| SecuritySession {
                id: SecuritySessionId(session.id),
                current: session.id == current_id,
                created: session.created_at,
                expires: session.expires_at,
            })
            .collect();
        let result = SecurityOverview {
            account_id,
            totp: user.two_factor_enabled.typed()?.unwrap_or(false),
            password,
            zitadel,
            provider_available: service.provider.is_some(),
            can_unlink_provider: accounts.len() > 1,
            passkey_count: passkeys.len(),
            keys,
            passkeys,
            sessions,
        };
        transaction
            .commit()
            .await
            .map_err(super::policy::database_error)?;
        Ok(result)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use loco_rs::controller::views::{ViewRenderer, engines::TeraView};
    use serde_json::json;

    fn render(password: bool, zitadel: bool, totp: bool, can_unlink_provider: bool) -> String {
        let created = Utc::now();
        let security = SecurityOverview {
            account_id: AccountId::from_database(1),
            password,
            zitadel,
            totp,
            provider_available: true,
            can_unlink_provider,
            passkey_count: 2,
            keys: vec![],
            passkeys: vec![
                SecurityPasskey {
                    id: SecurityPasskeyId(2),
                    name: Some("Named test key".into()),
                    created,
                },
                SecurityPasskey {
                    id: SecurityPasskeyId(3),
                    name: None,
                    created,
                },
            ],
            sessions: vec![SecuritySession {
                id: SecuritySessionId(4),
                current: true,
                created,
                expires: created,
            }],
        };
        let labels =
            crate::models::care::report_pdf::translations("en").expect("translations load");
        let security =
            crate::views::profile_security::context(&security, chrono_tz::UTC, &labels, created);
        TeraView::build().expect("templates load").render("profile/security.html", json!({
            "labels": labels,
            "security": security, "account_access": {"platform_admin":false,"support_access":false},
            "authenticity_token":"synthetic-csrf", "profile_path":"/profile", "slug":""
        })).expect("shared security panel renders")
    }

    #[test]
    fn provider_only_profile_keeps_status_and_add_password_without_unlink() {
        let html = render(false, true, false, false);
        assert!(html.contains("ZITADEL is linked."));
        assert!(html.contains("panel=reset"));
        assert!(!html.contains("value=\"unlink_provider\""));
        assert!(!html.contains("value=\"remove_password\""));
        assert!(!html.contains("panel=totp"));
        assert!(html.contains("data-passkey-id=\"2\""));
        assert!(html.contains("data-passkey-id=\"3\""));
        assert!(html.contains("Named test key"));
        assert!(html.contains("value=\"remove_passkey\""));
        assert!(html.contains("value=\"close_account\""));
        assert!(!html.contains("data-profile-form"));
    }

    #[test]
    fn password_profile_keeps_native_authenticator_and_multiple_method_controls() {
        let password_only = render(true, false, false, false);
        assert!(password_only.contains("panel=password"));
        assert!(password_only.contains("panel=totp"));
        assert!(!password_only.contains("value=\"remove_password\""));
        let multiple_methods = render(true, true, true, true);
        for action in ["unlink_provider", "remove_password"] {
            assert!(multiple_methods.contains(&format!("value=\"{action}\"")));
        }
        assert!(
            multiple_methods
                .contains("action=\"/account/security/totp/backup-codes?profile_flow=totp")
        );
        assert!(multiple_methods.contains("autocomplete=\"current-password\" required"));
        assert!(
            multiple_methods.contains("action=\"/account/security/totp/disable?profile_flow=totp")
        );
        assert!(!multiple_methods.contains("name=\"session_id\""));
        assert!(multiple_methods.contains("action=\"/logout\""));
    }
}
