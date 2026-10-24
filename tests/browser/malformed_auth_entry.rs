use playwright_rs::expect;
use playwright_rs::protocol::{AriaRole, Viewport};

use super::native_auth::capture_sizes;
use crate::harness::{self, named};

#[tokio::test]
#[ignore = "browser journeys run through task test:browser, not the shared --tests lanes"]
async fn malformed_auth_entry_recovers_through_public_forms() {
    harness::journey(
        Viewport {
            width: 1440,
            height: 1000,
        },
        |journey| {
            Box::pin(async move {
                let page = &journey.page;
                for (path, label, button, screenshot) in [
                    (
                        "/reset-password/confirm",
                        "Request a new password reset link",
                        "Send reset link",
                        "incomplete-password-reset",
                    ),
                    (
                        "/verify-account-confirm?callbackURL=private-callback-marker",
                        "Request a new verification email",
                        "Resend verification email",
                        "incomplete-verification",
                    ),
                ] {
                    page.goto(&format!("{}{path}", journey.origin), None)
                        .await
                        .unwrap();
                    expect(page.get_by_role(AriaRole::Heading, named("Incomplete link")))
                        .to_be_visible()
                        .await
                        .unwrap();
                    let title: String = page
                        .evaluate("() => document.title", None::<&()>)
                        .await
                        .unwrap();
                    assert_eq!(title, "Incomplete link | MedTracker");
                    capture_sizes(page, screenshot).await;
                    page.get_by_role(AriaRole::Link, named(label))
                        .click(None)
                        .await
                        .unwrap();
                    page.get_by_label("Email address", true)
                        .fill("persistence@example.test", None)
                        .await
                        .unwrap();
                    page.get_by_role(AriaRole::Button, named(button))
                        .click(None)
                        .await
                        .unwrap();
                    if button == "Send reset link" {
                        expect(page.get_by_role(AriaRole::Status, None))
                            .to_contain_text("we have sent a password reset link")
                            .await
                            .unwrap();
                    } else {
                        expect(page.get_by_role(AriaRole::Heading, named("Verify Account")))
                            .to_be_visible()
                            .await
                            .unwrap();
                    }
                }
            })
        },
    )
    .await;
}
