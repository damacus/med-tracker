use med_tracker::models::entities::{identity_session, identity_verification};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter, Set,
};

use super::*;

#[derive(Clone, Copy)]
enum Rejection {
    Stale,
    Expired,
    Reused,
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser, not the shared --tests lanes"]
async fn native_passkey_stale_challenge_is_denied_and_can_retry() {
    rejected_challenge(Rejection::Stale).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser, not the shared --tests lanes"]
async fn native_passkey_expired_challenge_is_denied_and_can_retry() {
    rejected_challenge(Rejection::Expired).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser, not the shared --tests lanes"]
async fn native_passkey_reused_challenge_is_denied_and_can_retry() {
    rejected_challenge(Rejection::Reused).await;
}

async fn rejected_challenge(rejection: Rejection) {
    harness::journey(Viewport { width: 1440, height: 1000 }, |journey| {
        Box::pin(async move {
            register_and_sign_out(journey).await;
            let page = &journey.page;
            let context = page.context().unwrap();
            hold_verification(page, &Value::Null).await;
            let button = page.get_by_role(AriaRole::Button, named("Sign in with a passkey"));
            button.focus().await.unwrap();
            page.keyboard().press("Enter", None).await.unwrap();
            page.wait_for_function("() => Boolean(window.heldPasskey)", None).await.unwrap();
            let held: Value = page.evaluate("() => window.heldPasskey", None::<&()>).await.unwrap();
            assert_eq!(held["signed"], true);
            assert_eq!(held["userVerified"], true);
            let original_cookie = context.cookies(None).await.unwrap().into_iter()
                .find(|cookie| cookie.name.ends_with("better-auth-passkey"))
                .expect("native challenge cookie");
            match rejection {
                Rejection::Stale => {
                    let fresh: Value = page.evaluate(r#"async () => {
                        const response = await fetch('/api/auth/passkey/generate-authenticate-options');
                        return {status: response.status, options: await response.json()};
                    }"#, None::<&()>).await.unwrap();
                    assert_eq!(fresh["status"], 200);
                    assert_ne!(fresh["options"]["challenge"], held["challenge"]);
                    let current_cookie = context.cookies(None).await.unwrap().into_iter()
                        .find(|cookie| cookie.name == original_cookie.name).unwrap();
                    assert_ne!(current_cookie.value, original_cookie.value);
                }
                Rejection::Expired => expire_challenge(journey, held["challenge"].as_str().unwrap()).await,
                Rejection::Reused => {}
            }
            let _: Value = page.evaluate("() => window.releasePasskey()", None::<&()>).await.unwrap();
            if matches!(rejection, Rejection::Reused) {
                expect(page.get_by_role(AriaRole::Button, named("Open household: Synthetic household")))
                    .to_be_visible().await.unwrap();
                page.goto(&format!("{}/account/security", journey.origin), None).await.unwrap();
                sign_out(page).await;
                hold_verification(page, &held["assertion"]).await;
                page.get_by_role(AriaRole::Button, named("Sign in with a passkey")).click(None).await.unwrap();
                page.wait_for_function("() => Boolean(window.heldPasskey)", None).await.unwrap();
                context.add_cookies(&[original_cookie]).await.unwrap();
                let _: Value = page.evaluate("() => window.releasePasskey()", None::<&()>).await.unwrap();
            }
            expect(page.get_by_role(AriaRole::Alert, None))
                .to_contain_text("Try again or use another sign-in method").await.unwrap();
            expect(page.get_by_role(AriaRole::Button, named("Sign in with a passkey")))
                .to_be_enabled().await.unwrap();
            let status: u16 = page.evaluate("() => window.passkeyStatus", None::<&()>).await.unwrap();
            assert_eq!(status, 400, "native challenge rejection");
            let error: String = page.evaluate("() => window.passkeyError", None::<&()>).await.unwrap();
            assert_eq!(error, match rejection {
                Rejection::Stale => "AUTHENTICATION_FAILED",
                Rejection::Expired | Rejection::Reused => "CHALLENGE_NOT_FOUND",
            });
            assert_signed_out(page).await;
            assert_eq!(identity_session::Entity::find()
                .filter(identity_session::Column::UserId.eq(71001))
                .filter(identity_session::Column::Active.eq(true))
                .count(&journey.admin).await.unwrap(), 0, "rejection created a stored session");
            let name = match rejection {
                Rejection::Stale => "stale-passkey-error",
                Rejection::Expired => "expired-passkey-error",
                Rejection::Reused => "reused-passkey-error",
            };
            capture_sizes(page, name).await;
            let _: Value = page.evaluate("() => window.restorePasskeyFetch()", None::<&()>).await.unwrap();
            let button = page.get_by_role(AriaRole::Button, named("Sign in with a passkey"));
            button.focus().await.unwrap();
            page.keyboard().press("Enter", None).await.unwrap();
            expect(page.get_by_role(AriaRole::Button, named("Open household: Synthetic household")))
                .to_be_visible().await.unwrap();
            assert!(native_session(page).await["session"].is_object());
        })
    }).await;
}

async fn hold_verification(page: &Page, replay: &Value) {
    let _: Value = page.evaluate(r#"replay => {
        const fetch = window.fetch.bind(window);
        window.restorePasskeyFetch = () => { window.fetch = fetch; };
        window.fetch = async (...args) => {
            if (!String(args[0]).endsWith('/passkey/verify-authentication')) return fetch(...args);
            const assertion = replay ?? JSON.parse(args[1].body).response;
            args[1].body = JSON.stringify({response: assertion});
            const decode = value => atob(value.replaceAll('-', '+').replaceAll('_', '/'));
            window.heldPasskey = {
                assertion,
                challenge: JSON.parse(decode(assertion.response.clientDataJSON)).challenge,
                signed: Boolean(assertion.response.signature),
                userVerified: Boolean(decode(assertion.response.authenticatorData).charCodeAt(32) & 4)
            };
            await new Promise(resolve => { window.releasePasskey = resolve; });
            const response = await fetch(...args);
            window.passkeyStatus = response.status;
            if (!response.ok) window.passkeyError = (await response.clone().json()).code;
            return response;
        };
    }"#, Some(replay)).await.unwrap();
}

async fn expire_challenge(journey: &harness::Journey, challenge: &str) {
    let records = identity_verification::Entity::find()
        .filter(identity_verification::Column::Value.contains(challenge))
        .all(&journey.admin)
        .await
        .unwrap();
    assert_eq!(records.len(), 1, "must identify only the held challenge");
    let mut record = records.into_iter().next().unwrap().into_active_model();
    record.expires_at = Set(chrono::Utc::now() - chrono::Duration::seconds(1));
    record.update(&journey.admin).await.unwrap();
}
