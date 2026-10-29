use crate::harness::{self, named};
use playwright_rs::expect;
use playwright_rs::protocol::{Animations, AriaRole, Page, ScreenshotOptions, Viewport};
use sea_orm::ConnectionTrait;

async fn capture(page: &Page, name: &str) {
    if page.locator("body.app-household").count().await.unwrap() > 0 {
        expect(page.locator("body[data-navigation-ready]"))
            .to_be_visible()
            .await
            .unwrap();
    }
    let overflowing = page.evaluate::<(), serde_json::Value>("() => [...document.querySelectorAll('body *')].filter(e => e.getBoundingClientRect().right > innerWidth).map(e => ({tag:e.tagName,classes:e.className,text:e.textContent.slice(0,80),right:e.getBoundingClientRect().right}))",None).await.unwrap();
    assert!(
        !page
            .evaluate::<(), bool>(
                "() => document.documentElement.scrollWidth > innerWidth",
                None
            )
            .await
            .unwrap(),
        "overflow: {name}: {overflowing}"
    );
    assert!(page.evaluate::<(), bool>("() => { const ids = [...document.querySelectorAll('[id]')].map(e=>e.id); return ids.length === new Set(ids).size; }", None).await.unwrap(), "duplicate ids: {name}");
    let directory = std::path::Path::new("docs/screenshots/shared-layout");
    std::fs::create_dir_all(directory).unwrap();
    page.screenshot_to_file(
        &directory.join(format!("admin-{name}.png")),
        ScreenshotOptions::builder()
            .animations(Animations::Disabled)
            .full_page(name.starts_with("overview") || name.contains("error"))
            .build(),
    )
    .await
    .unwrap();
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn administration_polish_overview_and_forms() {
    harness::journey(Viewport {width:1440,height:1000}, |journey| Box::pin(async move {
        let page=&journey.page;
        journey.admin.execute_unprepared("INSERT INTO household_invitations(id,household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,created_at,updated_at) VALUES(87801,72001,'synthetic-expired@example.test','member','polish-expired',74001,now()-interval '1 day',now(),now()); INSERT INTO security_audit_events(household_id,actor_account_id,actor_membership_id,event_type,metadata,audit_context,created_at,updated_at) VALUES(72001,71001,74001,'household_membership.role_updated','{\"secret\":\"private clinical sentinel\"}','{}',now(),now())").await.unwrap();
        harness::sign_in(journey).await;
        page.goto(&format!("{}/households/persistence-fixture/admin",journey.origin),None).await.unwrap();
        expect(page.locator("#expired-invitations")).to_contain_text("Expired invitations").await.unwrap();
        expect(page.locator(".admin-metrics")).to_contain_text("ACTIVE MEMBERS").await.unwrap();
        expect(page.locator(".admin-metrics")).to_contain_text("PENDING INVITATIONS").await.unwrap();
        expect(page.locator(".admin-metrics")).to_contain_text("PEOPLE NEEDING CARERS").await.unwrap();
        assert_eq!(page.locator(".admin-action-row").count().await.unwrap(),6);
        assert!(!page.locator("main").text_content().await.unwrap().unwrap().contains("private clinical sentinel"));
        expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
        harness::device_appearance(page, "dark").await;
        assert!(page.locator("html").get_attribute("data-theme").await.unwrap().unwrap().ends_with("-dark"));
        capture(page,"overview-dark-desktop").await;
        harness::device_appearance(page, "light").await;
        page.locator("#expired-invitations a").click(None).await.unwrap();
        expect(page.locator("#invitation-form")).to_be_visible().await.unwrap();
        page.get_by_label("Email",true).fill("synthetic-invite@example.test",None).await.unwrap();
        page.get_by_role(AriaRole::Button,named("Send invitation")).click(None).await.unwrap();
        expect(page.locator("main")).to_contain_text("Invitation email queued for delivery.").await.unwrap();
        for (path,label) in [("","overview"),("/users","members"),("/invitations","invitations"),("/household/edit","settings"),("/users/74001/edit","role"),("/carer_relationships","care"),("/carer_relationships/new","care-form"),("/audit_logs","audit")] {
            page.goto(&format!("{}/households/persistence-fixture/admin{path}",journey.origin),None).await.unwrap();
            expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
            if label == "overview" { expect(page.locator(".admin-activity")).to_contain_text("Invitation created").await.unwrap(); }
            for width in [1440,390,320] {
                page.set_viewport_size(Viewport {width,height:if width==1440 {1000}else{844}}).await.unwrap();
                capture(page,&format!("{label}-{width}")).await;
            }
            if label == "invitations" {
                let trigger = page.locator("button[data-dialog-open='cancel-invitation-87801']");
                trigger.click(None).await.unwrap();
                expect(page.locator("dialog[open]")).to_be_visible().await.unwrap();
                capture(page,"invitation-dialog-320").await;
                page.keyboard().press("Escape",None).await.unwrap();
                expect(trigger).to_be_focused().await.unwrap();
            }
        }
        let detail = page.locator("main table tbody tr:first-child a").get_attribute("href").await.unwrap().unwrap();
        page.goto(&format!("{}{detail}",journey.origin),None).await.unwrap();
        expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
        for width in [1440,390,320] {
            page.set_viewport_size(Viewport {width,height:if width==1440 {1000}else{844}}).await.unwrap();
            capture(page,&format!("audit-detail-{width}")).await;
        }
        page.goto(&format!("{}/households/persistence-fixture/admin/household/edit",journey.origin),None).await.unwrap();
        page.get_by_label("Household name",true).fill("   ",None).await.unwrap();
        page.get_by_role(AriaRole::Button,named("Save household")).click(None).await.unwrap();
        expect(page.locator("[role=alert]")).to_be_visible().await.unwrap();
        capture(page,"settings-error-320").await;
        page.get_by_label("Household name",true).fill("Synthetic renamed household",None).await.unwrap();
        page.get_by_role(AriaRole::Button,named("Save household")).click(None).await.unwrap();
        expect(page.get_by_label("Household name",true)).to_have_value("Synthetic renamed household").await.unwrap();
        page.goto(&format!("{}/households/persistence-fixture/admin/audit_logs?event_type=nonexistent",journey.origin),None).await.unwrap();
        expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
        expect(page.locator("main li")).to_contain_text("No events match these filters.").await.unwrap();
        page.evaluate::<(),bool>("() => { document.querySelector('main li').scrollIntoView({block:'center'}); return true; }",None).await.unwrap();
        capture(page,"audit-empty-320").await;
    })).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn administration_polish_platform_frames() {
    harness::journey(Viewport {width:1440,height:1000}, |journey| Box::pin(async move {
        let page=&journey.page;
        harness::sign_in(journey).await;
        journey.admin.execute_unprepared("INSERT INTO platform_admins(account_id,status,created_at,updated_at) VALUES(71001,'active',now(),now())").await.unwrap();
        for (path,label) in [("users","platform-users"),("settings","platform-settings"),("support","platform-support"),("owner-recovery","platform-recovery")] {
            page.goto(&format!("{}/platform/{path}",journey.origin),None).await.unwrap();
            expect(page.locator(".platform-page")).to_be_visible().await.unwrap();
            assert_eq!(page.locator(".app-sidebar").count().await.unwrap(),0);
            for width in [1440,390,320] {
                page.set_viewport_size(Viewport {width,height:if width==1440 {1000}else{844}}).await.unwrap();
                capture(page,&format!("{label}-{width}")).await;
            }
        }
    })).await;
}
