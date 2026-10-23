use playwright_rs::expect;
use playwright_rs::protocol::{AriaRole, Page, Viewport};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::Value;

use super::native_auth::{capture_sizes, capture_sizes_at};
use crate::harness::{self, named};

const NEW_PASSWORD: &str = "Native-browser-passphrase-42";

async fn mail_link(journey: &harness::Journey, subject: &str, address: &str) -> url::Url {
    let mail: Value = journey.admin.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT task_data FROM pg_loco_queue WHERE task_data->>'subject'=$1 AND task_data->>'to'=$2 ORDER BY created_at DESC LIMIT 1",
        [subject.into(), address.into()],
    )).await.unwrap().unwrap().try_get("", "task_data").unwrap();
    url::Url::parse(
        mail["text"]
            .as_str()
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap(),
    )
    .unwrap()
}

async fn visit_mail(page: &Page, origin: &str, link: &url::Url) {
    page.goto(
        &format!(
            "{origin}{}{}",
            link.path(),
            link.query()
                .map(|query| format!("?{query}"))
                .unwrap_or_default()
        ),
        None,
    )
    .await
    .unwrap();
}

async fn sign_in(page: &Page, origin: &str) {
    page.goto(&format!("{origin}/login"), None).await.unwrap();
    page.get_by_label("Email address", true)
        .fill("persistence@example.test", None)
        .await
        .unwrap();
    page.get_by_label("Password", true)
        .fill(NEW_PASSWORD, None)
        .await
        .unwrap();
    page.get_by_role(AriaRole::Button, named("Sign in"))
        .click(None)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser, not the shared --tests lanes"]
async fn native_auth_reset_key_mfa_backup_and_email_forms_complete() {
    harness::journey(Viewport {width:1440,height:1000}, |journey| Box::pin(async move {
        let page=&journey.page;
        page.goto(&format!("{}/reset-password",journey.origin),None).await.unwrap();
        page.get_by_label("Email address",true).fill("persistence@example.test",None).await.unwrap();
        capture_sizes(page,"password-reset-request").await;
        page.get_by_role(AriaRole::Button,named("Send reset link")).click(None).await.unwrap();
        expect(page.get_by_role(AriaRole::Status,None)).to_contain_text("we have sent a password reset link").await.unwrap();
        let reset=mail_link(journey,"Reset your password","persistence@example.test").await;
        visit_mail(page,&journey.origin,&reset).await;
        expect(page.get_by_label("New password",true)).to_be_visible().await.unwrap();
        capture_sizes(page,"password-reset-confirm").await;
        page.get_by_label("New password",true).fill(NEW_PASSWORD,None).await.unwrap();
        page.get_by_role(AriaRole::Button,named("Save password")).click(None).await.unwrap();
        expect(page.get_by_role(AriaRole::Heading,named("Sign in"))).to_be_visible().await.unwrap();
        sign_in(page,&journey.origin).await;
        expect(page.get_by_role(AriaRole::Button,named("Open household: Synthetic household"))).to_be_visible().await.unwrap();
        page.get_by_role(AriaRole::Button,named("Open household: Synthetic household")).click(None).await.unwrap();

        page.goto(&format!("{}/households/persistence-fixture/profile#security",journey.origin),None).await.unwrap();
        expect(page.locator("#profile-tab-security")).to_be_visible().await.unwrap();
        page.locator("#profile-tab-security").click(None).await.unwrap();
        expect(page.locator("#security")).to_be_visible().await.unwrap();
        capture_sizes_at(page,"profile-security",Some("#security h2")).await;
        page.locator("#security a[href='/account/security']").first().click(None).await.unwrap();
        expect(page.get_by_role(AriaRole::Heading,named("Account security"))).to_be_visible().await.unwrap();
        page.get_by_role(AriaRole::Link,named("Personal API keys")).click(None).await.unwrap();
        expect(page.get_by_role(AriaRole::Heading,named("Personal API keys"))).to_be_visible().await.unwrap();
        page.goto(&format!("{}/households/persistence-fixture/profile#security",journey.origin),None).await.unwrap();
        page.locator("#security a[href='/account/security/email']").click(None).await.unwrap();
        expect(page.get_by_label("New email address",true)).to_be_visible().await.unwrap();
        page.goto(&format!("{}/households/persistence-fixture/profile#security",journey.origin),None).await.unwrap();
        page.locator("#security a[href='/account/security/keys']").click(None).await.unwrap();
        page.get_by_label("Name",true).fill("Browser scoped key",None).await.unwrap();
        page.get_by_label("Synthetic household",true).check(None).await.unwrap();
        page.get_by_label("Read care records",true).check(None).await.unwrap();
        page.get_by_label("Expires in days",true).fill("30",None).await.unwrap();
        capture_sizes(page,"personal-key-create").await;
        page.get_by_role(AriaRole::Button,named("Create API key")).click(None).await.unwrap();
        expect(page.get_by_role(AriaRole::Heading,named("Save your API key"))).to_be_visible().await.unwrap();
        assert!(page.get_by_label("API key",true).input_value(None).await.unwrap().starts_with("medtracker_"));
        page.get_by_role(AriaRole::Link,named("Return to personal API keys")).click(None).await.unwrap();
        expect(page.get_by_role(AriaRole::Heading,named("Browser scoped key"))).to_be_visible().await.unwrap();

        page.goto(&format!("{}/account/security/totp",journey.origin),None).await.unwrap();
        page.get_by_label("Current password",true).fill(NEW_PASSWORD,None).await.unwrap();
        page.get_by_role(AriaRole::Button,named("Continue")).click(None).await.unwrap();
        expect(page.get_by_role(AriaRole::Heading,named("Save your backup codes"))).to_be_visible().await.unwrap();
        let secret=page.get_by_label("Setup key",true).input_value(None).await.unwrap();
        let backups=page.locator("ul[aria-label='Backup codes'] li").all_text_contents().await.unwrap();
        assert_eq!(backups.len(),10);
        capture_sizes(page,"authenticator-setup").await;
        let secret=String::from_utf8(base32::decode(base32::Alphabet::Rfc4648 {padding:false},&secret).unwrap()).unwrap();
        let code=better_auth::plugins::TwoFactorPlugin::new().generate_totp(&secret).unwrap();
        page.get_by_label("Authentication code",true).fill(&code,None).await.unwrap();
        page.get_by_label("I have saved my backup codes",true).check(None).await.unwrap();
        page.get_by_role(AriaRole::Button,named("Enable authenticator app")).click(None).await.unwrap();
        expect(page.get_by_role(AriaRole::Heading,named("Account security"))).to_be_visible().await.unwrap();
        expect(page.get_by_text("Authenticator app: enabled",true)).to_be_visible().await.unwrap();
        page.get_by_role(AriaRole::Button,named("Sign out")).click(None).await.unwrap();
        expect(page.get_by_role(AriaRole::Heading,named("Sign in"))).to_be_visible().await.unwrap();
        sign_in(page,&journey.origin).await;
        expect(page.get_by_role(AriaRole::Heading,named("Verify your identity"))).to_be_visible().await.unwrap();
        page.get_by_label("Backup code",true).fill("invalid-backup-code",None).await.unwrap();
        page.get_by_role(AriaRole::Button,named("Use backup code")).click(None).await.unwrap();
        expect(page.get_by_role(AriaRole::Alert,None)).to_contain_text("The code is invalid or expired").await.unwrap();
        capture_sizes(page,"backup-code-login").await;
        page.get_by_label("Backup code",true).fill(backups[0].trim(),None).await.unwrap();
        page.get_by_role(AriaRole::Button,named("Use backup code")).click(None).await.unwrap();
        expect(page.get_by_role(AriaRole::Button,named("Open household: Synthetic household"))).to_be_visible().await.unwrap();
        page.goto(&format!("{}/account/security/email",journey.origin),None).await.unwrap();
        page.get_by_label("New email address",true).fill("browser-changed@example.test",None).await.unwrap();
        capture_sizes(page,"change-email").await;
        page.get_by_role(AriaRole::Button,named("Continue")).click(None).await.unwrap();
        expect(page.get_by_role(AriaRole::Heading,named("Check your new email"))).to_be_visible().await.unwrap();
        let link=mail_link(journey,"Verify your new email address","browser-changed@example.test").await;
        visit_mail(page,&journey.origin,&link).await;
        expect(page.get_by_role(AriaRole::Heading,named("Verify your email address"))).to_be_visible().await.unwrap();
        capture_sizes(page,"change-email-confirm").await;
        page.get_by_role(AriaRole::Button,named("Verify and continue")).click(None).await.unwrap();
        expect(page.get_by_role(AriaRole::Heading,named("Account security"))).to_be_visible().await.unwrap();
        expect(page.get_by_text("browser-changed@example.test",true)).to_be_visible().await.unwrap();
        let mirrored=journey.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT a.email=u.email_address::text AS mirrored,a.two_factor_enabled FROM accounts a JOIN users u ON u.person_id=a.person_id WHERE a.id=71001")).await.unwrap().unwrap();
        assert!(mirrored.try_get::<bool>("","mirrored").unwrap());
        assert!(mirrored.try_get::<bool>("","two_factor_enabled").unwrap());
    })).await;
}
