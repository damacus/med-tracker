use playwright_rs::expect;
use playwright_rs::protocol::{Animations, AriaRole, ScreenshotOptions, Viewport};

use crate::harness::named;
use crate::{fixtures, harness};
use sea_orm::ConnectionTrait;

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
            journey.admin.execute_unprepared(
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
                 INSERT INTO public.users(id,person_id,email_address,password_digest,active,created_at,updated_at)
                 VALUES(77999,73999,'screenshot-other@example.test',crypt('password',gen_salt('bf',4)),true,now(),now());
                 INSERT INTO public.guardian_declarations(subject_person_id,guardian_person_id,origin_household_id,declared_by_account_id,subject_name_at_declaration)
                 VALUES(73994,73001,72001,71001,'Synthetic restore child');
                 INSERT INTO public.carer_relationships(carer_id,patient_id,household_id,active,created_at,updated_at,guardian_declaration_id)
                 SELECT 73001,73994,72001,false,now(),now(),id FROM public.guardian_declarations WHERE subject_person_id=73994"
            ).await.unwrap();
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
