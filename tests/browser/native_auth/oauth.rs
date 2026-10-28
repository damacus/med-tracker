use med_tracker::models::_entities::oauth_grants;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use super::*;

#[tokio::test]
#[ignore = "browser journeys run through task test:browser, not the shared --tests lanes"]
async fn native_passkey_sign_in_resumes_account_bound_mobile_oauth() {
    harness::journey(Viewport { width: 1440, height: 1000 }, |journey| {
        Box::pin(async move {
            register_and_sign_out(journey).await;
            let page = &journey.page;
            let callback = "io.damacus.medtracker:/oauth2redirect";
            page.add_init_script(r#"(() => {
                const submit = HTMLFormElement.prototype.submit;
                HTMLFormElement.prototype.submit = function () {
                    if (this.id === 'oauth-response-form' && this.action === 'io.damacus.medtracker:/oauth2redirect') {
                        window.nativeOAuthHandoff = {
                            action: this.action, method: this.method,
                            fields: Object.fromEntries(new FormData(this))
                        };
                        return;
                    }
                    return submit.call(this);
                };
            })();"#).await.unwrap();
            let state = uuid::Uuid::new_v4().to_string();
            let challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
            let fields = [
                ("client_id", "native-journey"),
                ("response_type", "code"),
                ("redirect_uri", callback),
                ("scope", "medtracker offline_access"),
                ("state", state.as_str()),
                ("code_challenge", challenge),
                ("code_challenge_method", "S256"),
            ];
            let query = serde_urlencoded::to_string(fields).unwrap();
            page.goto(&format!("{}/authorize?{query}", journey.origin), None).await.unwrap();
            expect(page.get_by_role(AriaRole::Heading, named("Welcome back")))
                .to_be_visible().await.unwrap();
            assert_signed_out(page).await;
            let button = page.get_by_role(AriaRole::Button, named("Sign in with a passkey"));
            button.focus().await.unwrap();
            page.keyboard().press("Enter", None).await.unwrap();
            expect(page.get_by_role(AriaRole::Heading, named("Authorise access")))
                .to_be_visible().await.unwrap();
            expect(page.get_by_text("Synthetic native journey", true)).to_be_visible().await.unwrap();
            let consent: Value = page.evaluate(r#"() => {
                const form = document.getElementById('authorize-form');
                return {fields: Object.fromEntries(new FormData(form)), text: form.parentElement.textContent};
            }"#, None::<&()>).await.unwrap();
            for (name, value) in fields.into_iter().filter(|(name, _)| *name != "scope") {
                assert_eq!(consent["fields"][name], value, "OAuth continuation changed {name}");
            }
            let text = consent["text"].as_str().unwrap();
            assert!(!text.contains("Household:"));
            assert!(!text.contains("Person:"));
            assert!(native_session(page).await["session"].is_object());
            capture_sizes(page, "passkey-mobile-oauth-consent").await;
            page.get_by_label("MedTracker data", true).check(None).await.unwrap();
            page.get_by_role(AriaRole::Button, named("Allow access")).click(None).await.unwrap();
            page.wait_for_function("() => Boolean(window.nativeOAuthHandoff)", None).await.unwrap();
            expect(page.get_by_role(AriaRole::Heading, named("Return to application")))
                .to_be_visible().await.unwrap();
            let handoff: Value = page.evaluate("() => window.nativeOAuthHandoff", None::<&()>).await.unwrap();
            assert_eq!(handoff["action"], callback);
            assert_eq!(handoff["method"], "post");
            assert_eq!(handoff["fields"]["state"], state);
            assert!(handoff["fields"]["code"].as_str().is_some_and(|code| !code.is_empty()));
            assert!(handoff["fields"]["error"].is_null());
            let grants = oauth_grants::Entity::find()
                .filter(oauth_grants::Column::OauthApplicationId.eq(75002))
                .all(&journey.admin).await.unwrap();
            assert_eq!(grants.len(), 1);
            let grant = grants.first().unwrap();
            assert_eq!(grant.account_id, 71001);
            assert_eq!(grant.client_kind, "mobile");
            assert_eq!(grant.household_membership_id, None);
            assert_eq!(grant.person_id, None);
            assert_eq!(grant.scopes, "medtracker");
            assert_eq!(grant.redirect_uri.as_deref(), Some(callback));
            assert_eq!(grant.code_challenge.as_deref(), Some(challenge));
            assert_eq!(grant.code_challenge_method.as_deref(), Some("S256"));
            capture_sizes(page, "passkey-mobile-oauth-handoff").await;
        })
    }).await;
}
