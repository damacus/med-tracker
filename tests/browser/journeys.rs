use playwright_rs::expect;
use playwright_rs::protocol::{AriaRole, Viewport};
use sea_orm::ConnectionTrait;
use sha2::Digest as _;
use std::path::Path;

use crate::harness::named;
use crate::{fixtures, harness};

#[tokio::test]
#[ignore = "browser journeys run through task test:browser, not the shared --tests lanes"]
async fn account_only_household_entry_works_with_keyboard_at_desktop_and_mobile_sizes() {
    harness::journey(Viewport { width: 1440, height: 1000 }, |journey| {
        Box::pin(async move {
            journey.admin.execute_unprepared("INSERT INTO people(id,name,person_type,has_capacity,created_at,updated_at) VALUES(73050,'Household entrant',0,true,now(),now()); INSERT INTO accounts(id,person_id,email,password_hash,status,preferences,created_at,updated_at) VALUES(71050,73050,'household-entry@example.test',crypt('password',gen_salt('bf',4)),2,'{\"time_zone\":\"Europe/London\"}',now(),now()); INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71050,now()); DELETE FROM app_settings; INSERT INTO app_settings(invite_only,created_at,updated_at) VALUES(true,now(),now())").await.unwrap();
            let page = &journey.page;
            page.goto(&format!("{}/login", journey.origin), None).await.unwrap();
            page.get_by_label("Email address", true).fill("household-entry@example.test", None).await.unwrap();
            page.get_by_label("Password", true).fill("password", None).await.unwrap();
            page.get_by_role(AriaRole::Button, named("Sign in")).click(None).await.unwrap();
            expect(page.get_by_role(AriaRole::Heading, named("Choose a household"))).to_be_visible().await.unwrap();
            expect(page.get_by_label("Household name", true)).to_be_visible().await.unwrap();
            page.screenshot_to_file(Path::new("docs/screenshots/household-entry-empty-desktop.png"), None).await.unwrap();
            page.set_viewport_size(Viewport { width: 390, height: 844 }).await.unwrap();
            page.screenshot_to_file(Path::new("docs/screenshots/household-entry-empty-mobile.png"), None).await.unwrap();
            page.get_by_label("Household name", true).fill(" ", None).await.unwrap();
            page.get_by_label("Household name", true).press("Enter", None).await.unwrap();
            expect(page.get_by_role(AriaRole::Status, None)).to_contain_text("Enter a household name").await.unwrap();
            page.screenshot_to_file(Path::new("docs/screenshots/household-entry-error-mobile.png"), None).await.unwrap();
            page.get_by_label("Household name", true).fill("Browser care home", None).await.unwrap();
            page.get_by_label("Household name", true).press("Enter", None).await.unwrap();
            expect(page.locator("#dashboard")).to_be_visible().await.unwrap();
            assert!(page.url().contains("/households/browser-care-home-"));
            page.screenshot_to_file(Path::new("docs/screenshots/household-entry-dashboard-mobile.png"), None).await.unwrap();
            page.set_viewport_size(Viewport { width: 1440, height: 1000 }).await.unwrap();
            page.screenshot_to_file(Path::new("docs/screenshots/household-entry-dashboard-desktop.png"), None).await.unwrap();
        })
    }).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser, not the shared --tests lanes"]
async fn account_only_invitation_opens_dashboard_at_desktop_and_mobile_sizes() {
    harness::journey(Viewport { width: 1440, height: 1000 }, |journey| {
        Box::pin(async move {
            journey.admin.execute_unprepared("DELETE FROM users WHERE person_id=73001; INSERT INTO people(id,name,person_type,has_capacity,created_at,updated_at) VALUES(73050,'Household entrant',0,true,now(),now()); INSERT INTO accounts(id,person_id,email,password_hash,status,created_at,updated_at) VALUES(71050,73050,'household-entry@example.test',crypt('password',gen_salt('bf',4)),2,now(),now()); INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71050,now())").await.unwrap();
            let invitation = "browser-account-only-invitation";
            let digest = hex::encode(sha2::Sha256::digest(invitation.as_bytes()));
            journey.admin.execute_raw(sea_orm::Statement::from_sql_and_values(sea_orm::DbBackend::Postgres,
                "INSERT INTO household_invitations(household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,created_at,updated_at) VALUES(72001,'household-entry@example.test','member',$1,74001,now()+interval '7 days',now(),now())",
                [digest.into()],
            )).await.unwrap();
            let page = &journey.page;
            page.goto(&format!("{}/login", journey.origin), None).await.unwrap();
            page.get_by_label("Email address", true).fill("household-entry@example.test", None).await.unwrap();
            page.get_by_label("Password", true).fill("password", None).await.unwrap();
            page.get_by_role(AriaRole::Button, named("Sign in")).click(None).await.unwrap();
            page.goto(&format!("{}/invitations/accept?token={invitation}", journey.origin), None).await.unwrap();
            expect(page.get_by_role(AriaRole::Heading, named("Accept invitation"))).to_be_visible().await.unwrap();
            page.screenshot_to_file(Path::new("docs/screenshots/household-invitation-desktop.png"), None).await.unwrap();
            page.set_viewport_size(Viewport { width: 390, height: 844 }).await.unwrap();
            page.screenshot_to_file(Path::new("docs/screenshots/household-invitation-mobile.png"), None).await.unwrap();
            page.get_by_role(AriaRole::Button, named("Accept invitation")).press("Enter", None).await.unwrap();
            expect(page.locator("#dashboard")).to_be_visible().await.unwrap();
            assert!(page.url().ends_with("/households/persistence-fixture/dashboard"));
            page.screenshot_to_file(Path::new("docs/screenshots/household-invitation-dashboard-mobile.png"), None).await.unwrap();
            page.set_viewport_size(Viewport { width: 1440, height: 1000 }).await.unwrap();
            page.screenshot_to_file(Path::new("docs/screenshots/household-invitation-dashboard-desktop.png"), None).await.unwrap();
        })
    }).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser, not the shared --tests lanes"]
async fn barcode_miss_flows_into_manual_medication_wizard() {
    let viewport = Viewport {
        width: 1440,
        height: 1000,
    };
    harness::journey(viewport, |journey| {
        Box::pin(async move {
            let page = &journey.page;
            harness::sign_in(journey).await;
            harness::open_care_section(page, "Medication Finder").await;
            page.get_by_label("Barcode", true)
                .fill("9999999999999", None)
                .await
                .unwrap();
            page.get_by_role(AriaRole::Button, named("Find barcode"))
                .click(None)
                .await
                .unwrap();
            expect(page.get_by_role(AriaRole::Status, None))
                .to_contain_text("No matching product")
                .await
                .unwrap();
            page.get_by_role(AriaRole::Link, named("Enter medication manually"))
                .click(None)
                .await
                .unwrap();
            expect(page.get_by_role(AriaRole::Heading, named("Add a New Medication")))
                .to_be_visible()
                .await
                .unwrap();
            expect(page.locator("[name=\"barcode\"]"))
                .to_have_value("9999999999999")
                .await
                .unwrap();
            page.get_by_label("Name", true)
                .fill("Scanner manual tablets", None)
                .await
                .unwrap();
            page.get_by_role(AriaRole::Button, named("Continue"))
                .click(None)
                .await
                .unwrap();
            expect(page.get_by_role(AriaRole::Heading, named("Dose and plan")))
                .to_be_visible()
                .await
                .unwrap();
            page.get_by_label("Dose", true)
                .fill("1", None)
                .await
                .unwrap();
            page.get_by_label("Unit", true)
                .select_option("tablet", None)
                .await
                .unwrap();
            page.get_by_role(AriaRole::Button, named("Review medication plan"))
                .click(None)
                .await
                .unwrap();
            page.get_by_role(AriaRole::Button, named("Continue"))
                .click(None)
                .await
                .unwrap();
            page.get_by_label("Starting Supply", true)
                .fill("28", None)
                .await
                .unwrap();
            page.get_by_role(AriaRole::Button, named("Continue"))
                .click(None)
                .await
                .unwrap();
            page.get_by_label("Warnings", true)
                .fill("Synthetic warning", None)
                .await
                .unwrap();
            page.get_by_role(AriaRole::Button, named("Save Medication"))
                .click(None)
                .await
                .unwrap();
            expect(page.get_by_role(AriaRole::Heading, named("Scanner manual tablets created!")))
                .to_be_visible()
                .await
                .unwrap();
            page.get_by_role(AriaRole::Link, named("Manage Dose Options"))
                .click(None)
                .await
                .unwrap();
            expect(page.locator("text=/Current supply: 28(\\.0+)?$/"))
                .to_be_visible()
                .await
                .unwrap();
            page.get_by_role(AriaRole::Link, named("Back to Medication"))
                .click(None)
                .await
                .unwrap();
            page.reload(None).await.unwrap();
            expect(page.get_by_role(AriaRole::Heading, named("Scanner manual tablets")))
                .to_be_visible()
                .await
                .unwrap();
            expect(page.get_by_text("Synthetic warning", true))
                .to_be_visible()
                .await
                .unwrap();
            harness::capture(page, "barcode-miss-desktop").await;
        })
    })
    .await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser, not the shared --tests lanes"]
async fn barcode_hit_resolves_existing_medicine_to_refill() {
    let viewport = Viewport {
        width: 390,
        height: 844,
    };
    harness::journey(viewport, |journey| {
        Box::pin(async move {
            let page = &journey.page;
            page.add_init_script(
                r#"window.__cameraStops = 0;
window.__barcodeScannerTestLibrary = { Html5Qrcode: class {
  static async getCameras() { return [{ id: 'synthetic-camera' }]; }
  async start(_camera, _settings, decoded) { window.__decodeBarcode = decoded; }
  async stop() { window.__cameraStops += 1; }
  clear() {}
} };"#,
            )
            .await
            .unwrap();
            harness::sign_in(journey).await;
            fixtures::seed_barcode_metadata(&journey.admin).await;
            page.get_by_role(AriaRole::Button, named("Scan stock"))
                .click(None)
                .await
                .unwrap();
            let dialog = page.get_by_role(AriaRole::Dialog, named("Scan stock"));
            expect(dialog.clone()).to_be_visible().await.unwrap();
            dialog
                .get_by_role(AriaRole::Button, named("Start Scanner"))
                .click(None)
                .await
                .unwrap();
            expect(dialog.get_by_text("Point the camera at the pack barcode.", false))
                .to_be_visible()
                .await
                .unwrap();
            dialog
                .get_by_role(AriaRole::Button, named("Close scanner"))
                .click(None)
                .await
                .unwrap();
            expect(dialog.clone()).to_be_hidden().await.unwrap();
            let stops: i64 = page
                .evaluate::<(), i64>("() => window.__cameraStops", None)
                .await
                .unwrap();
            assert_eq!(stops, 1);
            page.evaluate::<(), serde_json::Value>(
                "() => window.__decodeBarcode('1234567890123')",
                None,
            )
            .await
            .unwrap();
            expect(page.locator("#inventory-scan-barcode"))
                .to_have_value("")
                .await
                .unwrap();
            page.get_by_role(AriaRole::Button, named("Scan stock"))
                .click(None)
                .await
                .unwrap();
            dialog
                .get_by_label("Barcode", true)
                .fill("1234567890123", None)
                .await
                .unwrap();
            dialog
                .get_by_role(AriaRole::Button, named("Find barcode"))
                .click(None)
                .await
                .unwrap();
            expect(dialog.get_by_text("Medicine found in your household.", false))
                .to_be_visible()
                .await
                .unwrap();
            expect(dialog.get_by_role(AriaRole::Link, named("Refill this medicine")))
                .to_have_class_regex("btn-primary")
                .await
                .unwrap();
            expect(dialog.get_by_role(AriaRole::Button, named("Find barcode")))
                .to_have_class_regex("btn-outline")
                .await
                .unwrap();
            dialog
                .get_by_role(AriaRole::Link, named("Refill this medicine"))
                .click(None)
                .await
                .unwrap();
            expect(page.get_by_role(AriaRole::Dialog, named("Refill inventory")))
                .to_be_visible()
                .await
                .unwrap();
            harness::capture(page, "barcode-hit-mobile").await;
        })
    })
    .await;
}
