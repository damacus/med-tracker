use crate::accessibility_level_a::{assert_no_violations, audit};
use crate::harness::{self, named};
use med_tracker::models::_entities::{
    household_memberships, medication_takes, person_household_approvals, person_schedules,
    schedules,
};
use playwright_rs::expect;
use playwright_rs::protocol::{AriaRole, ScreenshotOptions, Viewport};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

const GUARDIAN_NOTE: &str = "Guardian private note Q7Z";
const PRIVATE_TAKE: &str = "7.77";
const DIRECTORY: &str = "docs/screenshots/care-sharing";

async fn create_dependent_plan(
    journey: &harness::Journey,
) -> (schedules::Model, person_schedules::Model) {
    let page = &journey.page;
    page.goto(
        &format!(
            "{}/households/persistence-fixture/people/73003/treatments/schedules/new",
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
        .fill("2", None)
        .await
        .unwrap();
    page.get_by_label("Dose unit", true)
        .select_option("tablet", None)
        .await
        .unwrap();
    page.get_by_label("Start date", true)
        .fill("2026-01-01", None)
        .await
        .unwrap();
    page.get_by_label("End date", true)
        .fill("2027-12-31", None)
        .await
        .unwrap();
    page.get_by_label("Notes", true)
        .fill(GUARDIAN_NOTE, None)
        .await
        .unwrap();
    page.get_by_role(AriaRole::Button, named("Add schedule"))
        .click(None)
        .await
        .unwrap();
    expect(page.locator("main"))
        .to_contain_text(GUARDIAN_NOTE)
        .await
        .unwrap();
    let origin = schedules::Entity::find()
        .filter(schedules::Column::PersonId.eq(73003))
        .filter(schedules::Column::HouseholdId.eq(72001))
        .one(&journey.admin)
        .await
        .unwrap()
        .unwrap();
    let plan = person_schedules::Entity::find()
        .filter(person_schedules::Column::PersonId.eq(73003))
        .one(&journey.admin)
        .await
        .unwrap()
        .unwrap();
    (origin, plan)
}

async fn accept_dependent_sharing(journey: &harness::Journey) {
    let page = &journey.page;
    page.goto(
        &format!(
            "{}/invitations/accept?token={}",
            journey.origin,
            crate::fixtures::browser::DEPENDENT_SHARING_TOKEN
        ),
        None,
    )
    .await
    .unwrap();
    page.get_by_label("Dependent whose care will be shared", true)
        .select_option("73003", None)
        .await
        .unwrap();
    page.locator("input[name=sharing_consent]")
        .check(None)
        .await
        .unwrap();
    page.get_by_role(AriaRole::Button, named("Accept invitation"))
        .click(None)
        .await
        .unwrap();
    expect(page.get_by_role(AriaRole::Heading, named("Care sharing approved")))
        .to_be_visible()
        .await
        .unwrap();
}

async fn foreign_sign_in(journey: &harness::Journey) {
    let page = &journey.page;
    page.context().unwrap().clear_cookies(None).await.unwrap();
    page.goto(&format!("{}/login", journey.origin), None)
        .await
        .unwrap();
    page.get_by_label("Email address", true)
        .fill("foreign@example.test", None)
        .await
        .unwrap();
    page.get_by_label("Password", true)
        .fill("password", None)
        .await
        .unwrap();
    page.get_by_role(AriaRole::Button, named("Sign in"))
        .click(None)
        .await
        .unwrap();
    page.get_by_role(
        AriaRole::Button,
        named("Open household: Synthetic foreign household"),
    )
    .click(None)
    .await
    .unwrap();
}

async fn seed_private_take(journey: &harness::Journey, schedule_id: i64) {
    medication_takes::ActiveModel {
        household_id: Set(72001),
        schedule_id: Set(Some(schedule_id)),
        dose_amount: Set(Some(PRIVATE_TAKE.parse().unwrap())),
        dose_unit: Set(Some("tablet".into())),
        taken_at: Set(Some(chrono::Utc::now().naive_utc())),
        taken_from_medication_id: Set(Some(80001)),
        portable_id: Set("synthetic-private-take-73003".into()),
        client_uuid: Set(Some("private-take-marker".into())),
        created_at: Set(chrono::Utc::now().naive_utc()),
        updated_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&journey.admin)
    .await
    .unwrap();
}

fn api_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap()
}

async fn bind_stock(origin: &str, plan: &str, medication: i64, revision: i64) -> reqwest::Response {
    api_client()
        .post(format!(
            "{origin}/api/v1/households/72002/shared_schedules/{plan}/stock_binding"
        ))
        .bearer_auth(crate::fixtures::browser::FOREIGN_BIND_TOKEN)
        .json(&serde_json::json!({
            "medication_id": medication,
            "clinical_revision": revision
        }))
        .send()
        .await
        .unwrap()
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn dependent_sharing_consent_approval_and_manager_stock_binding() {
    harness::journey(
        Viewport {
            width: 1440,
            height: 1000,
        },
        |journey| {
            Box::pin(async move {
                crate::fixtures::browser::seed_dependent_sharing(&journey.admin).await;
                harness::sign_in(journey).await;
                let page = &journey.page;
                let mut reports = Vec::new();
                // The current guardian creates the dependent's person-owned plan
                // in the origin household, including private clinical notes.
                let (origin_schedule, plan) = create_dependent_plan(journey).await;
                // Dependent-sharing acceptance shows the consent wording and the
                // dependent chooser before the approval page confirms sharing.
                page.goto(
                    &format!(
                        "{}/invitations/accept?token={}",
                        journey.origin,
                        crate::fixtures::browser::DEPENDENT_SHARING_TOKEN
                    ),
                    None,
                )
                .await
                .unwrap();
                expect(page.locator("main"))
                    .to_contain_text("I am a current declared guardian and approve sharing this dependent’s current and future care plans, pause status and latest dose safety status with this household. Private notes and detailed history stay private. Withdrawal or departure ends this approval.")
                    .await
                    .unwrap();
                page.get_by_label("Dependent whose care will be shared", true)
                    .select_option("73003", None)
                    .await
                    .unwrap();
                reports.push(audit(page, "dependent-consent").await);
                let directory = std::path::Path::new(DIRECTORY);
                std::fs::create_dir_all(directory).unwrap();
                for (name, width, height) in [("desktop", 1440, 1000), ("mobile", 390, 844)] {
                    page.set_viewport_size(Viewport { width, height })
                        .await
                        .unwrap();
                    assert!(
                        !page
                            .evaluate::<(), bool>(
                                "() => document.documentElement.scrollWidth > innerWidth",
                                None
                            )
                            .await
                            .unwrap(),
                        "overflow on consent {name}"
                    );
                    page.screenshot_to_file(
                        &directory.join(format!("dependent-consent-{name}.png")),
                        ScreenshotOptions::builder()
                            .full_page(name == "desktop")
                            .build(),
                    )
                    .await
                    .unwrap();
                }
                // The consent checkbox is keyboard-operable: focus plus Space
                // toggles it without any pointer interaction.
                let consent = page.locator("input[name=sharing_consent]");
                consent.focus().await.unwrap();
                page.keyboard().press(" ", None).await.unwrap();
                expect(consent).to_be_checked().await.unwrap();
                page.get_by_role(AriaRole::Button, named("Accept invitation"))
                    .click(None)
                    .await
                    .unwrap();
                expect(page.get_by_role(AriaRole::Heading, named("Care sharing approved")))
                    .to_be_visible()
                    .await
                    .unwrap();
                expect(page.get_by_role(AriaRole::Status, None))
                    .to_contain_text("current and future plans are shared")
                    .await
                    .unwrap();
                reports.push(audit(page, "dependent-approved").await);
                for (name, width, height) in [("desktop", 1440, 1000), ("mobile", 390, 844)] {
                    page.set_viewport_size(Viewport { width, height })
                        .await
                        .unwrap();
                    page.screenshot_to_file(
                        &directory.join(format!("dependent-approved-{name}.png")),
                        ScreenshotOptions::builder()
                            .full_page(name == "desktop")
                            .build(),
                    )
                    .await
                    .unwrap();
                }
                let approval = person_household_approvals::Entity::find()
                    .filter(person_household_approvals::Column::PersonId.eq(73003))
                    .filter(person_household_approvals::Column::HouseholdId.eq(72002))
                    .one(&journey.admin)
                    .await
                    .unwrap()
                    .expect("dependent sharing approval");
                assert!(approval.ended_at.is_none());
                assert!(approval.guardian_declaration_id.is_some());
                assert_eq!(approval.approved_by_account_id, 71001);
                let shared_member = household_memberships::Entity::find()
                    .filter(household_memberships::Column::PersonId.eq(73003))
                    .filter(household_memberships::Column::HouseholdId.eq(72002))
                    .one(&journey.admin)
                    .await
                    .unwrap()
                    .expect("shared household membership");
                assert_eq!(shared_member.status.as_str(), "active");
                // Denial surface: an unknown token renders the invalid-link page.
                page.goto(
                    &format!("{}/invitations/accept?token=bogus-share", journey.origin),
                    None,
                )
                .await
                .unwrap();
                expect(page.locator("main"))
                    .to_contain_text("Invitation link unavailable")
                    .await
                    .unwrap();
                reports.push(audit(page, "invitation-invalid").await);
                // The current guardian edits the plan cross-household without a
                // second consent: the same approval covers current and future
                // clinical revisions.
                page.goto(
                    &format!(
                        "{}/households/persistence-fixture/people/73003/treatments/schedules/{}/edit",
                        journey.origin, origin_schedule.id
                    ),
                    None,
                )
                .await
                .unwrap();
                page.get_by_label("Dose amount", true)
                    .fill("3", None)
                    .await
                    .unwrap();
                page.get_by_role(AriaRole::Button, named("Edit schedule"))
                    .click(None)
                    .await
                    .unwrap();
                let revised = person_schedules::Entity::find_by_id(&plan.id)
                    .one(&journey.admin)
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!(revised.clinical_revision, 2);
                let client = api_client();
                let shared = client
                    .get(format!(
                        "{}/api/v1/households/72002/shared_schedules",
                        journey.origin
                    ))
                    .bearer_auth(crate::fixtures::browser::FOREIGN_BIND_TOKEN)
                    .send()
                    .await
                    .unwrap();
                assert_eq!(shared.status(), reqwest::StatusCode::OK);
                let shared: serde_json::Value = shared.json().await.unwrap();
                let plans = shared["data"].as_array().unwrap();
                assert_eq!(plans.len(), 1, "one approval covers the shared plan");
                assert_eq!(plans[0]["id"].as_str().unwrap(), plan.id);
                assert_eq!(plans[0]["clinical_revision"].as_i64().unwrap(), 2);
                for private in ["notes", "pause_reason", "pause_note", "dose_history"] {
                    assert!(
                        plans[0].get(private).is_none(),
                        "shared plan must not expose {private}"
                    );
                }
                // A non-member account cannot even list the shared plans.
                let denied = client
                    .get(format!(
                        "{}/api/v1/households/72002/shared_schedules",
                        journey.origin
                    ))
                    .bearer_auth(crate::fixtures::browser::GUARDIAN_SESSION_TOKEN)
                    .send()
                    .await
                    .unwrap();
                assert!(matches!(denied.status().as_u16(), 401 | 403));
                // Exact-identity stock binding by the household manager: stale
                // revision and foreign-household stock are denied first.
                for (medication, revision) in [(80002_i64, 99_i64), (80001, 2)] {
                    let rejected =
                        bind_stock(&journey.origin, &plan.id, medication, revision).await;
                    assert!(
                        !rejected.status().is_success(),
                        "medication {medication} revision {revision} must not bind"
                    );
                }
                let bound = bind_stock(&journey.origin, &plan.id, 80002, 2).await;
                assert_eq!(bound.status(), reqwest::StatusCode::OK);
                let bound: serde_json::Value = bound.json().await.unwrap();
                assert!(bound["data"]["id"].as_i64().is_some());
                let approvals = person_household_approvals::Entity::find()
                    .filter(person_household_approvals::Column::PersonId.eq(73003))
                    .filter(person_household_approvals::Column::HouseholdId.eq(72002))
                    .all(&journey.admin)
                    .await
                    .unwrap();
                assert_eq!(
                    approvals.len(),
                    1,
                    "one approval covers current and future plans"
                );
                seed_private_take(journey, origin_schedule.id).await;
                assert_no_violations(&reports);
            })
        },
    )
    .await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn foreign_bound_schedule_hides_private_data_and_keeps_stock_local() {
    harness::journey(
        Viewport {
            width: 1440,
            height: 1000,
        },
        |journey| {
            Box::pin(async move {
                crate::fixtures::browser::seed_dependent_sharing(&journey.admin).await;
                harness::sign_in(journey).await;
                let page = &journey.page;
                let mut reports = Vec::new();
                let (origin_schedule, plan) = create_dependent_plan(journey).await;
                accept_dependent_sharing(journey).await;
                // Bind the plan to the foreign household's own stock at the
                // current clinical revision so it renders as a local schedule.
                let bound = bind_stock(&journey.origin, &plan.id, 80002, 1).await;
                assert_eq!(bound.status(), reqwest::StatusCode::OK);
                let bound: serde_json::Value = bound.json().await.unwrap();
                let bound_id = bound["data"]["id"].as_i64().unwrap();
                seed_private_take(journey, origin_schedule.id).await;
                // The household manager sees the bound schedule against local
                // stock without clinical affordances or foreign private data.
                foreign_sign_in(journey).await;
                page.goto(
                    &format!(
                        "{}/households/foreign-fixture/people/73003/treatments",
                        journey.origin
                    ),
                    None,
                )
                .await
                .unwrap();
                expect(page.locator("main"))
                    .to_contain_text("Synthetic foreign medicine")
                    .await
                    .unwrap();
                for denied in [
                    "Edit schedule",
                    "Add schedule",
                    "Assign medication",
                    GUARDIAN_NOTE,
                ] {
                    expect(page.locator("main"))
                        .not()
                        .to_contain_text(denied)
                        .await
                        .unwrap();
                }
                reports.push(audit(page, "foreign-bound-schedule").await);
                let directory = std::path::Path::new(DIRECTORY);
                std::fs::create_dir_all(directory).unwrap();
                for (name, width, height) in [("desktop", 1440, 1000), ("mobile", 390, 844)] {
                    page.set_viewport_size(Viewport { width, height })
                        .await
                        .unwrap();
                    assert!(
                        !page
                            .evaluate::<(), bool>(
                                "() => document.documentElement.scrollWidth > innerWidth",
                                None
                            )
                            .await
                            .unwrap(),
                        "overflow on foreign treatments {name}"
                    );
                    page.screenshot_to_file(
                        &directory.join(format!("foreign-bound-schedule-{name}.png")),
                        ScreenshotOptions::builder()
                            .full_page(name == "desktop")
                            .build(),
                    )
                    .await
                    .unwrap();
                }
                // Direct edit access is denied; detailed origin dose history and
                // private notes never project to the foreign household.
                page.goto(
                    &format!(
                        "{}/households/foreign-fixture/people/73003/treatments/schedules/{bound_id}/edit",
                        journey.origin
                    ),
                    None,
                )
                .await
                .unwrap();
                expect(page.locator("body"))
                    .to_contain_text("You are not authorized to perform this action")
                    .await
                    .unwrap();
                page.goto(
                    &format!(
                        "{}/households/foreign-fixture/people/73003/treatments/schedules/{bound_id}/doses",
                        journey.origin
                    ),
                    None,
                )
                .await
                .unwrap();
                expect(page.locator("body"))
                    .not()
                    .to_contain_text(PRIVATE_TAKE)
                    .await
                    .unwrap();
                expect(page.locator("body"))
                    .not()
                    .to_contain_text(GUARDIAN_NOTE)
                    .await
                    .unwrap();
                // Local stock selection stays visibly separate from clinical
                // permission: the record-dose dialog offers the household's own
                // stock even though the manager cannot edit the plan.
                page.goto(
                    &format!(
                        "{}/households/foreign-fixture/dashboard?dashboard_person_id=73003&record_medication_id=80002&record_source_type=schedule&record_source_id={bound_id}&record_person_id=73003",
                        journey.origin
                    ),
                    None,
                )
                .await
                .unwrap();
                expect(page.locator("#record-dose-dialog[open]"))
                    .to_be_visible()
                    .await
                    .unwrap();
                expect(page.locator(format!("label[for=stock-source-schedule-{bound_id}]")))
                    .to_contain_text("Stock source")
                    .await
                    .unwrap();
                expect(page.locator(format!(
                    "#stock-source-schedule-{bound_id} option[value=\"80002\"]"
                )))
                .to_contain_text("Synthetic foreign cabinet")
                .await
                .unwrap();
                reports.push(audit(page, "foreign-stock-source-dialog").await);
                for (name, width, height) in [("desktop", 1440, 1000), ("mobile", 390, 844)] {
                    page.set_viewport_size(Viewport { width, height })
                        .await
                        .unwrap();
                    assert!(
                        !page
                            .evaluate::<(), bool>(
                                "() => document.documentElement.scrollWidth > innerWidth",
                                None
                            )
                            .await
                            .unwrap(),
                        "overflow on stock dialog {name}"
                    );
                    page.screenshot_to_file(
                        &directory.join(format!("foreign-stock-source-{name}.png")),
                        ScreenshotOptions::builder()
                            .full_page(name == "desktop")
                            .build(),
                    )
                    .await
                    .unwrap();
                }
                // Keyboard dialog handling: Escape dismisses the modal; the
                // close handler strips the record_* params and returns focus.
                page.keyboard().press("Escape", None).await.unwrap();
                expect(page.locator("#record-dose-dialog"))
                    .to_be_hidden()
                    .await
                    .unwrap();
                // Denial surface: a signed-in account with no current guardian
                // declarations sees the no-dependents alert.
                page.goto(
                    &format!(
                        "{}/invitations/accept?token={}",
                        journey.origin,
                        crate::fixtures::browser::FOREIGN_SHARING_TOKEN
                    ),
                    None,
                )
                .await
                .unwrap();
                expect(page.locator("p[role=alert]"))
                    .to_contain_text("No dependent has current eligible guardian authority")
                    .await
                    .unwrap();
                reports.push(audit(page, "sharing-no-dependents").await);
                assert_no_violations(&reports);
            })
        },
    )
    .await;
}
