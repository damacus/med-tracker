use super::access::{HouseholdScope, TenantTransaction};
use super::entities::membership;
use super::identity::credentials::CredentialProvenance;
use serde_json::{Value, json};

pub(crate) fn household(
    tenant: &TenantTransaction,
    provenance: Option<&CredentialProvenance>,
) -> Value {
    for_request(tenant, &tenant.scope().request_id, provenance)
}

pub(crate) fn for_request(
    tenant: &TenantTransaction,
    request_id: &str,
    provenance: Option<&CredentialProvenance>,
) -> Value {
    build(tenant.scope(), tenant.membership(), request_id, provenance)
}

fn build(
    scope: &HouseholdScope,
    membership: &membership::Model,
    request_id: &str,
    provenance: Option<&CredentialProvenance>,
) -> Value {
    let mut context = json!({
        "actor_account_id": scope.actor.account_id,
        "actor_person_id": membership.person_id,
        "actor_membership_id": membership.id,
        "household_id": scope.household_id,
        "request_id": request_id,
        "active_role": membership.role,
        "permissions_version": membership.permissions_version
    });
    if let Some(provenance) = provenance {
        context["authentication_method"] = json!(provenance.method.label());
        context["session_reference"] = json!(format!(
            "{}:{}",
            provenance.method.reference_prefix(),
            provenance.reference
        ));
    }
    context
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::access::Actor;
    use crate::models::identity::credentials::CredentialMethod;
    use serde_json::json;

    fn fixture() -> (HouseholdScope, membership::Model) {
        let now = chrono::NaiveDate::from_ymd_opt(2026, 10, 12)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        (
            HouseholdScope {
                actor: Actor { account_id: 71 },
                household_id: 72,
                request_id: "scope-request".into(),
            },
            membership::Model {
                id: 74,
                household_id: 72,
                person_id: 73,
                permissions_version: 5,
                role: "administrator".into(),
                status: "active".into(),
                revoked_at: None,
                joined_at: now,
                created_at: now,
                updated_at: now,
            },
        )
    }

    #[test]
    fn household_audit_preserves_actor_and_explicit_request_without_credentials() {
        let (scope, membership) = fixture();
        let context = build(&scope, &membership, "operation-request", None);
        assert_eq!(
            context,
            json!({"actor_account_id":71,"actor_person_id":73,"actor_membership_id":74,"household_id":72,"request_id":"operation-request","active_role":"administrator","permissions_version":5})
        );
    }

    #[test]
    fn household_audit_preserves_each_credential_method_and_reference_prefix() {
        let (scope, membership) = fixture();
        for (method, label, prefix) in [
            (
                CredentialMethod::PersonalApiKey,
                "personal_api_key",
                "personal_api_key",
            ),
            (CredentialMethod::ApiSession, "api_session", "api_session"),
            (
                CredentialMethod::ApiAppToken,
                "api_app_token",
                "api_app_token",
            ),
            (CredentialMethod::OauthGrant, "oauth", "oauth_grant"),
            (
                CredentialMethod::BrowserSession,
                "browser_session",
                "browser_session",
            ),
        ] {
            let provenance = CredentialProvenance {
                method,
                reference: "synthetic-reference".into(),
            };
            let context = build(&scope, &membership, &scope.request_id, Some(&provenance));
            assert_eq!(context["authentication_method"], label);
            assert_eq!(
                context["session_reference"],
                format!("{prefix}:synthetic-reference")
            );
            assert_eq!(context["request_id"], "scope-request");
            assert_eq!(context["permissions_version"], 5);
        }
    }
}
