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
            let fields: Value = page.evaluate(r#"() => [...(document.querySelector('dialog.profile-flow-dialog[open]') || document.querySelector('main')).querySelectorAll('label[for]')].map(label => {
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
                expect(page.get_by_label("Passkey name", true))
                    .to_be_visible()
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

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn profile_dialogs_keep_failed_proofs_and_synthetic_secret_results_in_place() {
    harness::journey(Viewport { width: 390, height: 844 }, |journey| Box::pin(async move {
        harness::sign_in(journey).await;
        let page = &journey.page;
        let profile = format!("{}/households/persistence-fixture/profile#security", journey.origin);
        page.goto(&profile, None).await.unwrap();
        expect(page.locator("[data-profile-dialogs-ready=true]")).to_be_visible().await.unwrap();
        for panel in ["email", "password", "totp", "passkey", "keys"] {
            let opener = page.locator(format!("#security [data-profile-flow-entry={panel}]"));
            opener.focus().await.unwrap();
            page.keyboard().press("Enter", None).await.unwrap();
            expect(page.locator(format!("dialog[open] [data-flow-kind={panel}]"))).to_be_visible().await.unwrap();
            assert_eq!(page.url(), profile);
            assert!(!page.evaluate::<(), bool>("() => document.querySelector('dialog[open]').scrollWidth > innerWidth", None).await.unwrap());
            page.keyboard().press("Escape", None).await.unwrap();
            expect(opener).to_be_focused().await.unwrap();
        }
        page.locator("#security [data-profile-flow-entry=password]").click(None).await.unwrap();
        page.get_by_label("Current password", true).fill("incorrect", None).await.unwrap();
        page.get_by_label("New password", true).fill("Synthetic-unaccepted-passphrase-42", None).await.unwrap();
        page.get_by_role(AriaRole::Button, named("Continue")).click(None).await.unwrap();
        expect(page.locator("dialog[open] [data-flow-outcome=invalid] [role=alert]")).to_be_visible().await.unwrap();
        assert_eq!(page.url(), profile);
        page.keyboard().press("Escape", None).await.unwrap();
        expect(page.locator("[data-profile-flow-inline] #current-change-password")).to_have_value("").await.unwrap();
        page.locator("[data-profile-flow-reopen]").click(None).await.unwrap();
        expect(page.locator("dialog[open] [data-flow-outcome=invalid]")).to_be_visible().await.unwrap();
        page.keyboard().press("Escape", None).await.unwrap();
        page.locator("#security [data-profile-flow-entry=keys]").click(None).await.unwrap();
        expect(page.locator("dialog[open] #key-name")).to_be_visible().await.unwrap();
        page.get_by_label("Name", true).fill("Synthetic display only", None).await.unwrap();
        page.evaluate::<(), ()>(r#"() => {
            const original = window.fetch.bind(window);
            window.__profileMutationCount = 0;
            window.fetch = (url, options) => {
                if (options?.method !== 'POST' || !String(url).includes('/account/security/keys')) return original(url, options);
                window.__profileMutationCount += 1;
                return new Promise(resolve => { window.__finishProfileMutation = () => resolve({
                    url: String(url), headers: new Headers({'Content-Type':'text/html'}),
                    text: async () => '<section data-profile-flow-root data-flow-kind="keys" data-flow-outcome="secret-result" data-flow-secret="true" data-flow-profile="/households/persistence-fixture/profile"><h1>Synthetic one-time result</h1><label for="synthetic-key">Synthetic key</label><input id="synthetic-key" readonly value="synthetic-secret-visible-once"><a href="/households/persistence-fixture/profile#security" data-profile-flow-done>Done</a></section>'
                }); });
            };
        }"#, None).await.unwrap();
        page.get_by_role(AriaRole::Button, named("Create API key")).click(None).await.unwrap();
        expect(page.locator("dialog[open]")).to_have_attribute("aria-busy", "true").await.unwrap();
        page.keyboard().press("Escape", None).await.unwrap();
        expect(page.locator("dialog[open]")).to_be_visible().await.unwrap();
        page.evaluate::<(), ()>("() => window.__finishProfileMutation()", None).await.unwrap();
        expect(page.locator("dialog[open] [data-flow-outcome=secret-result]")).to_be_visible().await.unwrap();
        expect(page.get_by_label("Synthetic key", true)).to_have_value("synthetic-secret-visible-once").await.unwrap();
        page.locator("dialog [data-profile-flow-done]").click(None).await.unwrap();
        assert!(!page.evaluate::<(), bool>("() => document.documentElement.outerHTML.includes('synthetic-secret-visible-once')", None).await.unwrap());
        assert_eq!(page.evaluate::<(), u32>("() => window.__profileMutationCount", None).await.unwrap(), 1);
        assert_eq!(page.url(), profile);
    })).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn profile_dialogs_native_fallback_survives_disabled_download_and_initialisation_failure() {
    use playwright_rs::protocol::BrowserContextOptions;
    harness::journey(Viewport { width: 1440, height: 1000 }, |journey| Box::pin(async move {
        harness::sign_in(journey).await;
        let current = journey.page.context().unwrap();
        for failure in ["disabled", "download", "initialisation"] {
            let context = current.browser().unwrap().new_context_with_options(BrowserContextOptions::builder().javascript_enabled(failure != "disabled").viewport(Viewport { width: 390, height: 844 }).build()).await.unwrap();
            context.add_cookies(&current.cookies(None).await.unwrap()).await.unwrap();
            let page = context.new_page().await.unwrap();
            if failure == "download" {
                page.route("**/static/js/profile-dialogs.js*", |route| async move { route.abort(None).await }).await.unwrap();
            }
            if failure == "initialisation" {
                page.add_init_script("document.addEventListener('DOMContentLoaded', () => {}); Object.defineProperty(HTMLDialogElement.prototype, 'showModal', {value: undefined});").await.unwrap();
            }
            page.goto(&format!("{}/households/persistence-fixture/profile#security", journey.origin), None).await.unwrap();
            page.locator("#security [data-profile-flow-entry=email]").click(None).await.unwrap();
            expect(page.get_by_role(AriaRole::Heading, named("My Profile"))).to_be_visible().await.unwrap();
            expect(page.get_by_label("New email address", true)).to_be_visible().await.unwrap();
            assert!(page.url().contains("/profile?panel=email"));
            expect(page.locator("[data-profile-flow-root] > [data-profile-flow-done]")).to_have_attribute("href", "/households/persistence-fixture/profile#security").await.unwrap();
            context.close().await.unwrap();
        }
    })).await;
}
