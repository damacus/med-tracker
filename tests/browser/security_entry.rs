use playwright_rs::{
    expect,
    protocol::{AriaRole, Page, Viewport},
};
use sea_orm::{ActiveModelTrait, Set};
use serde_json::Value;

use crate::harness::{self, named};

async fn inspect_form(page: &Page, name: &str) {
    for appearance in ["light", "dark"] {
        harness::device_appearance(page, appearance).await;
        for width in [1440, 390] {
            page.set_viewport_size(Viewport {
                width,
                height: 1000,
            })
            .await
            .unwrap();
            let fields: Value = page.evaluate(r#"() => [...document.querySelectorAll('main label[for]')].map(label => {
                const input = document.getElementById(label.htmlFor);
                const range = document.createRange();
                range.selectNode(label.firstChild);
                const text = range.getBoundingClientRect();
                const field = input.getBoundingClientRect();
                return {name: label.htmlFor, stacked: text.bottom <= field.top, fullWidth: field.width >= label.getBoundingClientRect().width - 1};
            })"#, None::<&()>).await.unwrap();
            for field in fields.as_array().unwrap() {
                assert_eq!(
                    field["stacked"], true,
                    "{name}/{appearance}/{width}: {field}"
                );
                assert_eq!(
                    field["fullWidth"], true,
                    "{name}/{appearance}/{width}: {field}"
                );
            }
            assert!(
                !fields.as_array().unwrap().is_empty(),
                "{name} has no labelled fields"
            );
            assert!(
                !page
                    .evaluate::<(), bool>(
                        "() => document.documentElement.scrollWidth > innerWidth",
                        None
                    )
                    .await
                    .unwrap()
            );
        }
        if appearance == "dark" && matches!(name, "password" | "totp" | "passkey" | "support-proof")
        {
            super::native_auth::capture_sizes(page, &format!("security-{name}-{appearance}")).await;
        }
    }
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn security_entry_forms_group_fields_and_name_support_proof() {
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
                    &format!("{}/account/security/password", journey.origin),
                    None,
                )
                .await
                .unwrap();
                inspect_form(page, "password").await;
                page.get_by_label("Current password", true)
                    .fill("incorrect-password", None)
                    .await
                    .unwrap();
                page.get_by_label("New password", true)
                    .fill("Security-entry-passphrase-42", None)
                    .await
                    .unwrap();
                page.get_by_role(AriaRole::Button, named("Continue"))
                    .click(None)
                    .await
                    .unwrap();
                expect(page.get_by_role(AriaRole::Alert, None))
                    .to_contain_text("Password change is unavailable")
                    .await
                    .unwrap();
                inspect_form(page, "password-error").await;
                page.goto(&format!("{}/account/security/totp", journey.origin), None)
                    .await
                    .unwrap();
                inspect_form(page, "totp").await;
                page.get_by_label("Current password", true)
                    .fill("incorrect-password", None)
                    .await
                    .unwrap();
                page.get_by_role(AriaRole::Button, named("Continue"))
                    .click(None)
                    .await
                    .unwrap();
                expect(page.get_by_role(AriaRole::Alert, None))
                    .to_contain_text("Check your current password")
                    .await
                    .unwrap();
                inspect_form(page, "totp-error").await;
                page.goto(&format!("{}/account/security", journey.origin), None)
                    .await
                    .unwrap();
                page.get_by_role(AriaRole::Link, named("Add passkey"))
                    .click(None)
                    .await
                    .unwrap();
                inspect_form(page, "passkey").await;
                let now = chrono::Utc::now().naive_utc();
                med_tracker::models::_entities::platform_admins::ActiveModel {
                    account_id: Set(71001),
                    status: Set("active".into()),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                }
                .insert(&journey.admin)
                .await
                .unwrap();
                page.goto(&format!("{}/platform/support", journey.origin), None)
                    .await
                    .unwrap();
                page.get_by_label("Household", true)
                    .select_option("72001", None)
                    .await
                    .unwrap();
                page.get_by_label("Reason", true)
                    .fill("Synthetic form review", None)
                    .await
                    .unwrap();
                page.get_by_role(AriaRole::Button, named("Request support access"))
                    .click(None)
                    .await
                    .unwrap();
                expect(
                    page.get_by_role(AriaRole::Heading, named("Confirm support access request")),
                )
                .to_be_visible()
                .await
                .unwrap();
                let title: String = page
                    .evaluate("() => document.title", None::<&()>)
                    .await
                    .unwrap();
                assert_eq!(title, "Confirm support access request | MedTracker");
                expect(page.get_by_role(AriaRole::Button, named("Confirm support access request")))
                    .to_be_visible()
                    .await
                    .unwrap();
                inspect_form(page, "support-proof").await;
                page.get_by_label("Current password", true)
                    .fill("password", None)
                    .await
                    .unwrap();
                page.get_by_role(AriaRole::Button, named("Confirm support access request"))
                    .click(None)
                    .await
                    .unwrap();
                expect(page.get_by_role(AriaRole::Heading, named("Support access")))
                    .to_be_visible()
                    .await
                    .unwrap();
                expect(page.get_by_text("Synthetic form review", true))
                    .to_be_visible()
                    .await
                    .unwrap();
            })
        },
    )
    .await;
}
