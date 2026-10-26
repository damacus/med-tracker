use med_tracker::models::_entities::{
    household_memberships, households, oauth_grants, security_audit_events,
};
use playwright_rs::expect;
use playwright_rs::protocol::{Animations, AriaRole, FulfillOptions, ScreenshotOptions, Viewport};
use sea_orm::{ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use std::path::Path;

use crate::harness::{self, named};

#[tokio::test]
#[ignore = "browser journeys run through task test:browser, not the shared --tests lanes"]
async fn integration_consent_desktop_and_mobile() {
    harness::journey(
        Viewport {
            width: 1440,
            height: 1000,
        },
        |journey| {
            Box::pin(async move {
                households::Entity::insert(households::ActiveModel {
                    id: Set(72050),
                    created_by_account_id: Set(Some(71001)),
                    name: Set("Second care home".into()),
                    slug: Set("second-care-home".into()),
                    timezone: Set("Europe/London".into()),
                    created_at: Set(chrono::Utc::now().naive_utc()),
                    updated_at: Set(chrono::Utc::now().naive_utc()),
                    ..Default::default()
                })
                .exec(&journey.admin)
                .await
                .unwrap();
                household_memberships::Entity::insert(household_memberships::ActiveModel {
                    id: Set(74050),
                    household_id: Set(72050),
                    person_id: Set(73001),
                    role: Set("administrator".into()),
                    joined_at: Set(chrono::Utc::now().naive_utc()),
                    created_at: Set(chrono::Utc::now().naive_utc()),
                    updated_at: Set(chrono::Utc::now().naive_utc()),
                    ..Default::default()
                })
                .exec(&journey.admin)
                .await
                .unwrap();
                let page = &journey.page;
                let query = serde_urlencoded::to_string([
                    ("client_id", "synthetic-client"),
                    ("response_type", "code"),
                    ("redirect_uri", "https://example.test/callback"),
                    ("scope", "patient/*.read"),
                    ("state", "consent-reference"),
                    (
                        "code_challenge",
                        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
                    ),
                    ("code_challenge_method", "S256"),
                ])
                .unwrap();
                page.goto(&format!("{}/authorize?{query}", journey.origin), None)
                    .await
                    .unwrap();
                expect(page.get_by_role(AriaRole::Heading, named("Welcome back")))
                    .to_be_visible()
                    .await
                    .unwrap();
                page.get_by_label("Email address", true)
                    .fill("persistence@example.test", None)
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
                expect(
                    page.get_by_role(AriaRole::Button, named("Open household: Second care home")),
                )
                .to_be_visible()
                .await
                .unwrap();
                page.get_by_role(AriaRole::Button, named("Open household: Second care home"))
                    .click(None)
                    .await
                    .unwrap();
                expect(page.get_by_role(AriaRole::Heading, named("Authorise access")))
                    .to_be_visible()
                    .await
                    .unwrap();
                expect(page.get_by_text("Household: Second care home", true))
                    .to_be_visible()
                    .await
                    .unwrap();
                for (label, width, height) in [("desktop", 1440, 1000), ("mobile", 390, 844)] {
                    page.set_viewport_size(Viewport { width, height })
                        .await
                        .unwrap();
                    let path = Path::new("docs/screenshots/oauth-consent")
                        .join(format!("final-{label}.png"));
                    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                    page.screenshot_to_file(
                        &path,
                        ScreenshotOptions::builder()
                            .animations(Animations::Disabled)
                            .full_page(true)
                            .build(),
                    )
                    .await
                    .unwrap();
                }
                page.route("https://example.test/callback**", |route| async move {
                    route
                        .fulfill(
                            FulfillOptions::builder()
                                .content_type("text/html")
                                .body_string("<h1>Synthetic OAuth callback</h1>")
                                .build(),
                        )
                        .await
                })
                .await
                .unwrap();
                page.get_by_label("Read patient records", true)
                    .check(None)
                    .await
                    .unwrap();
                page.get_by_role(AriaRole::Button, named("Allow access"))
                    .click(None)
                    .await
                    .unwrap();
                expect(page.get_by_role(AriaRole::Heading, named("Synthetic OAuth callback")))
                    .to_be_visible()
                    .await
                    .unwrap();
                let grants = oauth_grants::Entity::find()
                    .filter(oauth_grants::Column::OauthApplicationId.eq(75001))
                    .filter(oauth_grants::Column::HouseholdMembershipId.eq(74050))
                    .all(&journey.admin)
                    .await
                    .unwrap();
                assert_eq!(grants.len(), 1);
                assert_eq!(grants[0].person_id, Some(73001));
                assert_eq!(grants[0].permissions_version, Some(1));
                assert_eq!(grants[0].scopes, "patient/*.read");
                assert!(grants[0].code.is_some());
                let audits = security_audit_events::Entity::find()
                    .filter(
                        security_audit_events::Column::EventType.eq("smart_oauth.consent_granted"),
                    )
                    .filter(security_audit_events::Column::HouseholdId.eq(72050))
                    .all(&journey.admin)
                    .await
                    .unwrap();
                assert_eq!(audits.len(), 1);
            })
        },
    )
    .await;
}
