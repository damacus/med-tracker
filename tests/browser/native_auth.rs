use playwright_rs::expect;
use playwright_rs::protocol::{Animations, AriaRole, Page, ScreenshotOptions, Viewport};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::{Value, json};

use crate::harness::{self, named};

async fn passkey_cancelled_sign_in_can_retry(viewport: Viewport, size: &'static str) {
    harness::journey(viewport, |journey| {
        Box::pin(async move {
            let page = &journey.page;
            let context = page.context().unwrap();
            let cdp = context.new_cdp_session(page).await.unwrap();
            cdp.send("WebAuthn.enable", Some(json!({"enableUI": false})))
                .await
                .unwrap();
            cdp.send(
                "WebAuthn.addVirtualAuthenticator",
                Some(json!({"options": {
                    "protocol": "ctap2",
                    "transport": "internal",
                    "hasResidentKey": true,
                    "hasUserVerification": true,
                    "isUserVerified": true,
                    "automaticPresenceSimulation": true
                }})),
            )
            .await
            .unwrap();

            harness::sign_in(journey).await;
            page.goto(&format!("{}/account/security", journey.origin), None)
                .await
                .unwrap();
            page.get_by_role(AriaRole::Link, named("Add passkey"))
                .click(None)
                .await
                .unwrap();
            page.get_by_label("Passkey name", true)
                .fill("Cancellation retry key", None)
                .await
                .unwrap();
            page.get_by_role(AriaRole::Button, named("Register Passkey"))
                .click(None)
                .await
                .unwrap();
            expect(page.get_by_text("Passkey: Cancellation retry key", true))
                .to_be_visible()
                .await
                .unwrap();
            page.get_by_role(AriaRole::Button, named("Sign out"))
                .click(None)
                .await
                .unwrap();
            expect(page.get_by_role(AriaRole::Heading, named("Sign in")))
                .to_be_visible()
                .await
                .unwrap();
            let _: Value = page
                .evaluate(
                    r#"() => {
                    const get = navigator.credentials.get.bind(navigator.credentials);
                    let cancel = true;
                    navigator.credentials.get = options => {
                        if (cancel) {
                            cancel = false;
                            return Promise.reject(new DOMException('Cancelled', 'NotAllowedError'));
                        }
                        return get(options);
                    };
                    const fetch = window.fetch.bind(window);
                    window.passkeyVerifyRequests = 0;
                    window.fetch = (...args) => {
                        if (String(args[0]).endsWith('/passkey/verify-authentication')) {
                            window.passkeyVerifyRequests += 1;
                        }
                        return fetch(...args);
                    };
                }"#,
                    None::<&()>,
                )
                .await
                .unwrap();

            let button = page.get_by_role(AriaRole::Button, named("Sign in with a passkey"));
            button.click(None).await.unwrap();
            expect(page.get_by_role(AriaRole::Alert, None))
                .to_contain_text("Try again or use another sign-in method")
                .await
                .unwrap();
            let enabled: bool = page
                .evaluate(
                    "() => !document.querySelector('[data-passkey-login]').disabled",
                    None::<&()>,
                )
                .await
                .unwrap();
            assert!(
                enabled,
                "cancelled sign-in left the passkey button disabled"
            );
            let attempts: u64 = page
                .evaluate("() => window.passkeyVerifyRequests", None::<&()>)
                .await
                .unwrap();
            assert_eq!(attempts, 0, "cancelled ceremony submitted an assertion");
            let session = native_session(page).await;
            assert!(
                session.is_null() || session["session"].is_null(),
                "cancelled ceremony created a session"
            );
            let directory = std::path::Path::new("docs/screenshots/native-auth");
            std::fs::create_dir_all(directory).unwrap();
            page.screenshot_to_file(
                &directory.join(format!("passkey-cancelled-{size}.png")),
                ScreenshotOptions::builder()
                    .animations(Animations::Disabled)
                    .full_page(true)
                    .build(),
            )
            .await
            .unwrap();

            button.click(None).await.unwrap();
            expect(page.get_by_role(
                AriaRole::Button,
                named("Open household: Synthetic household"),
            ))
            .to_be_visible()
            .await
            .unwrap();
            let session = native_session(page).await;
            assert!(
                !session["session"].is_null(),
                "retry did not create a session"
            );
        })
    })
    .await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser, not the shared --tests lanes"]
async fn native_passkey_cancelled_sign_in_can_retry_desktop() {
    passkey_cancelled_sign_in_can_retry(
        Viewport {
            width: 1440,
            height: 1000,
        },
        "desktop",
    )
    .await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser, not the shared --tests lanes"]
async fn native_passkey_cancelled_sign_in_can_retry_mobile() {
    passkey_cancelled_sign_in_can_retry(
        Viewport {
            width: 390,
            height: 844,
        },
        "mobile",
    )
    .await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser, not the shared --tests lanes"]
async fn native_passkey_registers_signs_in_and_rejects_signed_unverified_assertion() {
    harness::journey(Viewport { width: 1440, height: 1000 }, |journey| {
        Box::pin(async move {
            let page = &journey.page;
            let context = page.context().unwrap();
            let cdp = context.new_cdp_session(page).await.unwrap();
            cdp.send("WebAuthn.enable", Some(json!({"enableUI":false}))).await.unwrap();
            let authenticator = cdp.send("WebAuthn.addVirtualAuthenticator", Some(json!({"options":{
                "protocol":"ctap2", "transport":"internal", "hasResidentKey":true,
                "hasUserVerification":true, "isUserVerified":true,
                "automaticPresenceSimulation":true
            }}))).await.unwrap();
            let authenticator_id = authenticator["result"]["authenticatorId"].as_str().unwrap_or_else(|| panic!("missing CDP authenticator: {authenticator}"));
            page.add_init_script(r#"(() => {
                const original = window.fetch.bind(window);
                window.fetch = async (...args) => {
                    const response = await original(...args);
                    if (String(args[0]).endsWith('/passkey/verify-authentication')) {
                        const assertion = JSON.parse(args[1].body).response;
                        const encoded = assertion.response.authenticatorData.replaceAll('-', '+').replaceAll('_', '/');
                        const flags = atob(encoded).charCodeAt(32);
                        window.__passkeyAttempt = {status: response.status, userVerified: Boolean(flags & 4), signature: Boolean(assertion.response.signature)};
                    }
                    return response;
                };
            })();"#).await.unwrap();
            harness::sign_in(journey).await;
            page.goto(&format!("{}/account/security", journey.origin), None).await.unwrap();
            expect(page.get_by_role(AriaRole::Heading, named("Account security"))).to_be_visible().await.unwrap();
            capture_sizes(page, "account-security").await;
            page.get_by_role(AriaRole::Link, named("Add passkey")).click(None).await.unwrap();
            page.get_by_label("Passkey name", true).fill("Browser acceptance key", None).await.unwrap();
            capture_sizes(page, "register-passkey").await;
            page.get_by_role(AriaRole::Button, named("Register Passkey")).click(None).await.unwrap();
            expect(page.get_by_text("Passkey: Browser acceptance key", true)).to_be_visible().await.unwrap();
            let credentials = cdp.send("WebAuthn.getCredentials", Some(json!({"authenticatorId":authenticator_id}))).await.unwrap();
            assert_eq!(credentials["result"]["credentials"].as_array().unwrap().len(), 1);
            page.get_by_role(AriaRole::Button, named("Sign out")).click(None).await.unwrap();
            expect(page.get_by_role(AriaRole::Heading, named("Sign in"))).to_be_visible().await.unwrap();
            page.goto(&format!("{}/login", journey.origin), None).await.unwrap();
            capture_sizes(page, "sign-in").await;
            cdp.send("WebAuthn.setResponseOverrideBits", Some(json!({"authenticatorId":authenticator_id,"isBadUV":true}))).await.unwrap();
            page.get_by_role(AriaRole::Button, named("Sign in with a passkey")).click(None).await.unwrap();
            expect(page.get_by_role(AriaRole::Alert, None)).to_contain_text("Unable to use this passkey").await.unwrap();
            let denied: Value = page.evaluate("() => window.__passkeyAttempt", None::<&()>).await.unwrap();
            assert_eq!(denied["userVerified"], false);
            assert_eq!(denied["signature"], true);
            assert!(denied["status"].as_u64().unwrap() >= 400, "signed no-UV assertion reached the native verifier: {denied}");
            let session = native_session(page).await;
            assert!(session.is_null() || session["session"].is_null(), "denied assertion created a session");
            capture_sizes(page, "unverified-passkey-error").await;
            cdp.send("WebAuthn.setResponseOverrideBits", Some(json!({"authenticatorId":authenticator_id,"isBadUV":false}))).await.unwrap();
            page.get_by_role(AriaRole::Button, named("Sign in with a passkey")).click(None).await.unwrap();
            expect(page.get_by_role(AriaRole::Button, named("Open household: Synthetic household"))).to_be_visible().await.unwrap();
            page.goto(&format!("{}/account/security", journey.origin), None).await.unwrap();
            expect(page.get_by_role(AriaRole::Heading, named("Account security"))).to_be_visible().await.unwrap();
            expect(page.get_by_text("persistence@example.test", true)).to_be_visible().await.unwrap();
            expect(page.get_by_text("Passkey: Browser acceptance key", true)).to_be_visible().await.unwrap();
            let session = native_session(page).await;
            assert!(session["session"].is_object(), "verified credential did not create a native session");
            capture_sizes(page, "passkey-signed-in").await;
            closure_retry(journey).await;
        })
    }).await;
}

async fn closure_retry(journey: &harness::Journey) {
    let page = &journey.page;
    let original_session = native_session(page).await;
    let started: Value = page.evaluate(
        "async () => { const response = await fetch('/api/auth/security/operation/start', {method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({action:'close_account'})}); return {status:response.status,body:await response.json()}; }",
        None::<&()>,
    ).await.unwrap();
    assert_eq!(started["status"], 200);
    let operation = started["body"]["operation_id"].as_str().unwrap();
    let before = pending(journey, operation).await;
    journey.admin.execute_unprepared("CREATE FUNCTION native_closure_queue_failure() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic closure failure'; END $$; CREATE TRIGGER native_closure_queue_failure BEFORE INSERT ON pg_loco_queue FOR EACH ROW WHEN(NEW.task_data->>'subject'='Account closed') EXECUTE FUNCTION native_closure_queue_failure()").await.unwrap();
    let failed = confirm_closure(page, operation).await;
    assert_eq!(
        failed["status"], 500,
        "queue failure was not reached: {failed}"
    );
    let after = pending(journey, operation).await;
    let retained_session = native_session(page).await;
    assert_eq!(
        original_session["session"]["id"], retained_session["session"]["id"],
        "closure failure replaced or revoked the original browser session"
    );
    let status = journey
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status FROM accounts WHERE id=71001",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i32>("", "status")
        .unwrap();
    assert_eq!(status, 2, "closure failure changed account status");
    journey
        .admin
        .execute_unprepared("DROP TRIGGER native_closure_queue_failure ON pg_loco_queue")
        .await
        .unwrap();
    let retried = confirm_closure(page, operation).await;
    assert_eq!(
        retried["status"],
        200,
        "same-operation passkey retry failed; pending preserved={}: {retried}",
        before == after
    );
    assert_eq!(
        before, after,
        "closure failure rebound the original pending operation"
    );
    let closed = journey.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT status, NOT EXISTS(SELECT 1 FROM identity_verifications WHERE identifier=$1) AS consumed FROM accounts WHERE id=71001",
        [format!("password-change:{operation}").into()],
    )).await.unwrap().unwrap();
    assert_eq!(closed.try_get::<i32>("", "status").unwrap(), 3);
    assert!(closed.try_get::<bool>("", "consumed").unwrap());
    let session = native_session(page).await;
    assert!(
        session.is_null() || session["session"].is_null(),
        "closed account retained an authenticated browser session"
    );
}

async fn pending(journey: &harness::Journey, operation: &str) -> String {
    journey
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT value FROM identity_verifications WHERE identifier=$1",
            [format!("password-change:{operation}").into()],
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "value")
        .unwrap()
}

async fn confirm_closure(page: &Page, operation: &str) -> Value {
    page.evaluate(r#"async operation_id => {
        const send = (path, body) => fetch(path, {method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)});
        const challenge = await send('/api/auth/security/passkey/start', {operation_id});
        if (!challenge.ok) return {status:challenge.status, stage:'challenge'};
        const publicKey = PublicKeyCredential.parseRequestOptionsFromJSON(await challenge.json());
        publicKey.userVerification = 'required';
        const credential = await navigator.credentials.get({publicKey});
        const response = await send('/api/auth/security/passkey/confirm', {operation_id,response:credential.toJSON()});
        return {status:response.status,stage:'confirm'};
    }"#, Some(&operation)).await.unwrap()
}

async fn native_session(page: &Page) -> Value {
    let response: Value = page.evaluate(
        "async () => { const response = await fetch('/api/auth/get-session'); return {status: response.status, body: await response.json()}; }",
        None::<&()>,
    ).await.unwrap();
    assert_eq!(response["status"], 200, "native session endpoint failed");
    response["body"].clone()
}

pub(super) async fn capture_sizes(page: &Page, name: &str) {
    capture_sizes_at(page, name, None).await;
}

pub(super) async fn capture_sizes_at(page: &Page, name: &str, scroll_target: Option<&str>) {
    let directory = std::path::Path::new("docs/screenshots/native-auth");
    std::fs::create_dir_all(directory).unwrap();
    for (size, width, height) in [("desktop", 1440, 1000), ("mobile", 390, 844)] {
        page.set_viewport_size(Viewport { width, height })
            .await
            .unwrap();
        if let Some(selector) = scroll_target {
            page.locator(selector)
                .scroll_into_view_if_needed()
                .await
                .unwrap();
        }
        let fits: bool = page
            .evaluate(
                "() => document.documentElement.scrollWidth <= innerWidth",
                None::<&()>,
            )
            .await
            .unwrap();
        assert!(fits, "{name} overflows the {size} viewport");
        page.screenshot_to_file(
            &directory.join(format!("{name}-{size}.png")),
            ScreenshotOptions::builder()
                .animations(Animations::Disabled)
                .mask(vec![page.locator("input[type='password'], #totp-secret, [aria-label='Backup codes'], #created-api-key, #backup-code")])
                .mask_color("#d1d5db")
                .build(),
        )
        .await
        .unwrap();
    }
    page.set_viewport_size(Viewport {
        width: 1440,
        height: 1000,
    })
    .await
    .unwrap();
}
