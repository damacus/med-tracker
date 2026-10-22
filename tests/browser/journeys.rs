use playwright_rs::expect;
use playwright_rs::protocol::{Animations, AriaRole, ScreenshotOptions, Viewport};
use sea_orm::ConnectionTrait;
use sha2::Digest as _;
use std::path::Path;

use crate::harness::named;
use crate::{fixtures, harness};

async fn guardian_recovery_screen(viewport: Viewport, screenshot: &'static str) {
    harness::journey(viewport, |journey| {
        Box::pin(async move {
            journey.admin.execute_unprepared(
                "UPDATE public.household_memberships SET role='owner' WHERE id=74001;
                 INSERT INTO public.people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at)
                 VALUES(73994,'Synthetic recovery child','2020-01-01',1,false,now(),now());
                 INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at)
                 VALUES(74994,72001,73994,'member','active',now(),now(),now())"
            ).await.unwrap();
            harness::sign_in(journey).await;
            let page=&journey.page;
            page.goto(&format!("{}/households/persistence-fixture/admin/carer_relationships",journey.origin),None).await.unwrap();
            page.get_by_role(AriaRole::Link,named("Recover a minor's guardian")).click(None).await.unwrap();
            page.get_by_role(AriaRole::Link,named("Review recovery")).click(None).await.unwrap();
            expect(page.get_by_role(AriaRole::Heading,named("Recover guardian: Synthetic recovery child"))).to_be_visible().await.unwrap();
            expect(page.get_by_label("Eligible adult",true)).to_be_visible().await.unwrap();
            expect(page.get_by_label("I confirm that I want to appoint this adult as guardian.",true)).to_be_visible().await.unwrap();
            let path=std::path::Path::new("docs/screenshots/browser-journeys").join(format!("{screenshot}.png"));
            page.screenshot_to_file(&path,ScreenshotOptions::builder().animations(Animations::Disabled).full_page(true).build()).await.unwrap();
        })
    }).await;
}

async fn guardian_recovery_restore_screen(viewport: Viewport, screenshot: &'static str) {
    harness::journey(viewport, |journey| {
        Box::pin(async move {
            let scope = med_tracker::models::access::HouseholdScope {
                actor: med_tracker::models::access::Actor { account_id: 71001 },
                household_id: 72001,
                request_id: "synthetic-browser-guardian-restore".into(),
            };
            let fixture = med_tracker::models::access::begin(&journey.admin, &scope, None).await.unwrap();
            fixture.transaction().execute_unprepared(
                "UPDATE public.household_memberships SET role='owner' WHERE id=74001;
                 UPDATE public.people SET name='Linked guardian <care>' WHERE id=73001;
                 INSERT INTO public.people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at)
                 VALUES(73994,'Synthetic restore child','2020-01-01',1,false,now(),now()),
                       (73999,'Other eligible adult','1980-01-01',0,true,now(),now());
                 INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at)
                 VALUES(74994,72001,73994,'member','active',now(),now(),now()),
                       (74999,72001,73999,'member','active',now(),now(),now());
                 INSERT INTO public.accounts(id,person_id,email,status,created_at,updated_at)
                 VALUES(71999,73999,'screenshot-other@example.test',2,now(),now());
                 INSERT INTO public.users(id,person_id,email_address,active,created_at,updated_at)
                 VALUES(77999,73999,'screenshot-other@example.test',true,now(),now());
                 INSERT INTO public.guardian_declarations(subject_person_id,guardian_person_id,origin_household_id,declared_by_account_id,subject_name_at_declaration)
                 VALUES(73994,73001,72001,71001,'Synthetic restore child');
                 INSERT INTO public.carer_relationships(carer_id,patient_id,household_id,active,created_at,updated_at,guardian_declaration_id)
                 SELECT 73001,73994,72001,false,now(),now(),id FROM public.guardian_declarations WHERE subject_person_id=73994"
            ).await.unwrap();
            fixture.commit().await.unwrap();
            harness::sign_in(journey).await;
            let page=&journey.page;
            page.goto(&format!("{}/households/persistence-fixture/admin/carer_relationships/recover/73994",journey.origin),None).await.unwrap();
            expect(page.get_by_role(AriaRole::Heading,named("Recover guardian: Synthetic restore child"))).to_be_visible().await.unwrap();
            expect(page.get_by_text("Guardian who will regain access: Linked guardian <care>",true)).to_be_visible().await.unwrap();
            expect(page.get_by_label("I confirm that I want to restore this guardian's access.",true)).to_be_visible().await.unwrap();
            let path=std::path::Path::new("docs/screenshots/browser-journeys").join(format!("{screenshot}.png"));
            page.screenshot_to_file(&path,ScreenshotOptions::builder().animations(Animations::Disabled).full_page(true).build()).await.unwrap();
        })
    }).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser, not the shared --tests lanes"]
async fn guardian_recovery_restore_desktop() {
    guardian_recovery_restore_screen(
        Viewport {
            width: 1440,
            height: 1000,
        },
        "guardian-recovery-restore-desktop",
    )
    .await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser, not the shared --tests lanes"]
async fn guardian_recovery_restore_mobile() {
    guardian_recovery_restore_screen(
        Viewport {
            width: 390,
            height: 844,
        },
        "guardian-recovery-restore-mobile",
    )
    .await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser, not the shared --tests lanes"]
async fn guardian_recovery_desktop() {
    guardian_recovery_screen(
        Viewport {
            width: 1440,
            height: 1000,
        },
        "guardian-recovery-desktop",
    )
    .await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser, not the shared --tests lanes"]
async fn guardian_recovery_mobile() {
    guardian_recovery_screen(
        Viewport {
            width: 390,
            height: 844,
        },
        "guardian-recovery-mobile",
    )
    .await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser, not the shared --tests lanes"]
async fn account_only_household_entry_works_with_keyboard_at_desktop_and_mobile_sizes() {
    harness::journey(Viewport { width: 1440, height: 1000 }, |journey| {
        Box::pin(async move {
            journey.admin.execute_unprepared("INSERT INTO people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73050,'Household entrant','1990-01-01',0,true,now(),now()); INSERT INTO accounts(id,person_id,email,status,preferences,created_at,updated_at) VALUES(71050,73050,'household-entry@example.test',2,'{\"time_zone\":\"Europe/London\"}',now(),now()); INSERT INTO identity_onboarding(account_id,completed_at) VALUES(71050,now()); DELETE FROM app_settings; INSERT INTO app_settings(invite_only,created_at,updated_at) VALUES(true,now(),now())").await.unwrap();
            crate::fixtures::seed_native_password(&journey.admin, 71050).await;
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
            let long_unicode_name = format!("Care {}", "😀".repeat(51));
            page.get_by_label("Household name", true).fill(&long_unicode_name, None).await.unwrap();
            expect(page.get_by_label("Household name", true)).to_have_value(&long_unicode_name).await.unwrap();
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
            journey.admin.execute_unprepared("DELETE FROM users WHERE person_id=73001; INSERT INTO people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73050,'Household entrant','1990-01-01',0,true,now(),now()); INSERT INTO accounts(id,person_id,email,status,created_at,updated_at) VALUES(71050,73050,'household-entry@example.test',2,now(),now()); INSERT INTO identity_onboarding(account_id,completed_at) VALUES(71050,now())").await.unwrap();
            crate::fixtures::seed_native_password(&journey.admin, 71050).await;
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
async fn household_member_table_and_confirmation_at_desktop_and_mobile() {
    for (viewport, screenshot) in [
        (
            Viewport {
                width: 1440,
                height: 1000,
            },
            "members-desktop",
        ),
        (
            Viewport {
                width: 390,
                height: 844,
            },
            "members-mobile",
        ),
        (
            Viewport {
                width: 320,
                height: 844,
            },
            "members-narrow",
        ),
    ] {
        harness::journey(viewport, |journey| {
            Box::pin(async move {
                journey.admin.execute_unprepared("INSERT INTO people(id,name,person_type,has_capacity,created_at,updated_at) VALUES(73006,'Hidden person name',0,true,now(),now()),(73016,'Accountless dependant',1,false,now(),now()); INSERT INTO accounts(id,person_id,name,email,status,created_at,updated_at) VALUES(71006,73006,'Inactive member','inactive@example.test',2,now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74006,72001,73006,'member','suspended',now(),now(),now()),(74016,72001,73016,'member','suspended',now(),now(),now())").await.unwrap();
                harness::sign_in(journey).await;
                let page = &journey.page;
                page.goto(&format!("{}/households/persistence-fixture/admin/users?status=suspended", journey.origin), None).await.unwrap();
                expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
                expect(page.get_by_role(AriaRole::Heading, named("Members"))).to_be_visible().await.unwrap();
                expect(page.locator("tbody")).to_contain_text("Inactive member").await.unwrap();
                expect(page.locator("tbody")).to_contain_text("inactive@example.test").await.unwrap();
                expect(page.locator("tbody")).to_contain_text("Accountless dependant").await.unwrap();
                assert!(!page.evaluate::<(),bool>("() => document.documentElement.scrollWidth > innerWidth",None).await.unwrap());
                page.locator("summary[aria-label='Restore Inactive member']").click(None).await.unwrap();
                expect(page.get_by_role(AriaRole::Button, named("Confirm Restore"))).to_be_visible().await.unwrap();
                page.evaluate::<(),bool>("() => { window.scrollTo(0,0); return true; }",None).await.unwrap();
                page.screenshot_to_file(Path::new(&format!("docs/screenshots/browser-journeys/{screenshot}.png")),ScreenshotOptions::builder().animations(Animations::Disabled).full_page(true).build()).await.unwrap();
                page.get_by_role(AriaRole::Link,named("Edit household role for Accountless dependant")).click(None).await.unwrap();
                expect(page.get_by_role(AriaRole::Heading,named("Edit household role for Accountless dependant"))).to_be_visible().await.unwrap();
                expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
                page.evaluate::<(),bool>("() => { window.scrollTo(0,0); return true; }",None).await.unwrap();
                page.screenshot_to_file(Path::new(&format!("docs/screenshots/browser-journeys/{screenshot}-role.png")),ScreenshotOptions::builder().animations(Animations::Disabled).full_page(true).build()).await.unwrap();

            })
        }).await;
    }
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
            page.get_by_label("Person", true)
                .select_option("73001", None)
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
            page.goto(
                &format!(
                    "{}/households/persistence-fixture/people/73001/treatments",
                    journey.origin
                ),
                None,
            )
            .await
            .unwrap();
            expect(page.locator("main"))
                .to_contain_text("Scanner manual tablets")
                .await
                .unwrap();
        })
    })
    .await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser, not the shared --tests lanes"]
async fn treatment_forms_create_and_edit_without_losing_clinical_fields() {
    harness::journey(
        Viewport {
            width: 1440,
            height: 1000,
        },
        |journey| {
            Box::pin(async move {
                let page = &journey.page;
                harness::sign_in(journey).await;
                page.goto(
                    &format!(
                        "{}/households/persistence-fixture/people/73001/treatments/schedules/new",
                        journey.origin
                    ),
                    None,
                )
                .await
                .unwrap();
                page.get_by_label("Medication", true)
                    .select_option("80001", None)
                    .await
                    .unwrap();
                page.get_by_label("Dose amount", true)
                    .fill("0.10", None)
                    .await
                    .unwrap();
                page.get_by_label("Dose unit", true)
                    .select_option("tablet", None)
                    .await
                    .unwrap();
                page.get_by_label("Start date", true)
                    .fill("2026-10-22", None)
                    .await
                    .unwrap();
                page.get_by_label("End date", true)
                    .fill("2026-11-22", None)
                    .await
                    .unwrap();
                page.get_by_label("Times", true)
                    .fill("08:00, 20:00", None)
                    .await
                    .unwrap();
                page.get_by_role(AriaRole::Button, named("Add schedule"))
                    .click(None)
                    .await
                    .unwrap();
                let schedule = page.locator("section[aria-labelledby='schedules-heading'] article");
                expect(page.locator("section[aria-labelledby='schedules-heading'] article"))
                    .to_contain_text("0.1 tablet")
                    .await
                    .unwrap();
                expect(page.locator("section[aria-labelledby='schedules-heading'] article"))
                    .to_contain_text("08:00, 20:00")
                    .await
                    .unwrap();
                page.screenshot_to_file(
                    Path::new(
                        "docs/screenshots/browser-journeys/typed-treatment-schedule-desktop.png",
                    ),
                    None,
                )
                .await
                .unwrap();
                schedule
                    .get_by_role(AriaRole::Link, named("Edit schedule"))
                    .click(None)
                    .await
                    .unwrap();
                page.set_viewport_size(Viewport {
                    width: 390,
                    height: 844,
                })
                .await
                .unwrap();
                page.get_by_label("Notes", true)
                    .fill("Typed schedule edit", None)
                    .await
                    .unwrap();
                page.get_by_role(AriaRole::Button, named("Edit schedule"))
                    .click(None)
                    .await
                    .unwrap();
                expect(page.locator("section[aria-labelledby='schedules-heading'] article"))
                    .to_contain_text("Typed schedule edit")
                    .await
                    .unwrap();
                expect(page.locator("section[aria-labelledby='schedules-heading'] article"))
                    .to_contain_text("08:00, 20:00")
                    .await
                    .unwrap();
                page.screenshot_to_file(
                    Path::new(
                        "docs/screenshots/browser-journeys/typed-treatment-schedule-mobile.png",
                    ),
                    ScreenshotOptions::builder().full_page(true).build(),
                )
                .await
                .unwrap();
                let assignment =
                    page.locator("section[aria-labelledby='assignments-heading'] article");
                assignment
                    .get_by_role(AriaRole::Link, named("Edit assignment"))
                    .click(None)
                    .await
                    .unwrap();
                page.get_by_label("Notes", true)
                    .fill("Typed assignment edit", None)
                    .await
                    .unwrap();
                page.get_by_role(AriaRole::Button, named("Edit assignment"))
                    .click(None)
                    .await
                    .unwrap();
                expect(page.locator("section[aria-labelledby='assignments-heading'] article"))
                    .to_contain_text("Typed assignment edit")
                    .await
                    .unwrap();
            })
        },
    )
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
