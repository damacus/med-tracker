use crate::harness::{self, named};
use playwright_rs::expect;
use playwright_rs::protocol::{Animations, AriaRole, Page, ScreenshotOptions, Viewport};
use sea_orm::ConnectionTrait;

async fn capture(page: &Page, name: &str) {
    expect(page.locator("body[data-navigation-ready]"))
        .to_be_visible()
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
        "overflow: {name}"
    );
    let detail = name.starts_with("failure") || name.starts_with("empty");
    page.evaluate::<bool,bool>("detail => { if (detail) document.querySelector('.admin-invitations').scrollIntoView({block:'center'}); else window.scrollTo(0,0); return true; }",Some(&detail)).await.unwrap();
    let directory = std::path::Path::new("docs/screenshots/invitation-management");
    std::fs::create_dir_all(directory).unwrap();
    page.screenshot_to_file(
        &directory.join(format!("{name}.png")),
        ScreenshotOptions::builder()
            .animations(Animations::Disabled)
            .full_page(!detail && !name.starts_with("error"))
            .build(),
    )
    .await
    .unwrap();
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn invitation_management_search_delivery_and_keyboard_actions() {
    harness::journey(Viewport{width:1440,height:1000},|journey|Box::pin(async move {
        journey.admin.execute_unprepared("INSERT INTO household_invitations(id,household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,created_at,updated_at) VALUES(87801,72001,'maya@example.test','member','browser-invitation-maya',74001,now()+interval '7 days',now(),now()),(87802,72001,'alex@example.test','administrator','browser-invitation-alex',74001,now()+interval '7 days',now()-interval '1 day',now()),(87803,72001,'legacy@example.test','member','browser-invitation-legacy',74001,now()-interval '1 day',now()-interval '8 days',now()); INSERT INTO invitation_mail_attempts(household_id,invitation_id,original_invitation_id,sender_account_id,sender_membership_id,sender_permissions_version,token_digest,status,category,started_at,finished_at) VALUES(72001,87801,87801,71001,74001,1,'browser-invitation-maya','failed','uncertain',now()-interval '1 hour',now()-interval '30 minutes'); INSERT INTO invitation_mail_attempts(household_id,invitation_id,original_invitation_id,sender_account_id,sender_membership_id,sender_permissions_version,token_digest,status) VALUES(72001,87802,87802,71001,74001,1,'browser-invitation-alex','queued')").await.unwrap();
        harness::sign_in(journey).await;
        let page=&journey.page;
        let path=format!("{}/households/persistence-fixture/admin/invitations?lang=en",journey.origin);
        page.goto(&path,None).await.unwrap();
        expect(page.get_by_role(AriaRole::Button,named("Resend maya@example.test"))).to_be_visible().await.unwrap();
        expect(page.locator(".admin-invitations")).to_contain_text("Not tracked").await.unwrap();
        expect(page.locator(".admin-invitations")).to_contain_text("This attempt will not be sent again.").await.unwrap();
        for width in [1440,390,320] {
            page.set_viewport_size(Viewport{width,height:if width==1440{1000}else{844}}).await.unwrap();
            assert!(page.evaluate::<(),bool>("() => [...document.querySelectorAll('.admin-invitations .card-body')].every(card => { const a=card.querySelector('.invitation-actions')?.getBoundingClientRect(); const b=card.querySelector('.invitation-delivery')?.getBoundingClientRect(); return !a || !b || a.top >= b.bottom || b.top >= a.bottom || a.left >= b.right || b.left >= a.right; })",None).await.unwrap(),"delivery/actions overlap at {width}");
            capture(page,&format!("management-{width}")).await;
        }
        page.set_viewport_size(Viewport{width:1440,height:1000}).await.unwrap();
        page.locator(".app-sidebar [data-sidebar-account]").click(None).await.unwrap();
        page.get_by_role(AriaRole::Button,named("Appearance")).click(None).await.unwrap();
        page.locator("[data-appearance-choice=dark]").click(None).await.unwrap();
        page.locator("#appearance-dialog [data-dialog-close]").click(None).await.unwrap();
        capture(page,"management-dark-1440").await;
        page.locator(".app-sidebar [data-sidebar-account]").click(None).await.unwrap();
        page.get_by_role(AriaRole::Button,named("Appearance")).click(None).await.unwrap();
        page.locator("[data-appearance-choice=system]").click(None).await.unwrap();
        page.locator("#appearance-dialog [data-dialog-close]").click(None).await.unwrap();
        page.set_viewport_size(Viewport{width:320,height:844}).await.unwrap();
        page.get_by_label("Email contains",true).fill("MAYA",None).await.unwrap();
        page.get_by_label("Household role",true).nth(1).select_option("member",None).await.unwrap();
        page.get_by_label("Invitation status",true).select_option("pending",None).await.unwrap();
        page.get_by_role(AriaRole::Button,named("Apply filters")).press("Enter",None).await.unwrap();
        assert!(page.url().contains("q=MAYA"));
        capture(page,"failure-320").await;
        page.get_by_role(AriaRole::Button,named("Resend maya@example.test")).press("Enter",None).await.unwrap();
        expect(page.locator(".admin-invitations")).to_contain_text("Queued").await.unwrap();
        expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
        assert!(page.url().contains("q=MAYA"));
        assert!(page.url().contains("role=member"));
        assert!(page.url().contains("status=pending"));
        assert!(page.url().contains("lang=en"));
        let trigger=page.get_by_role(AriaRole::Button,named("Cancel maya@example.test"));
        trigger.press("Enter",None).await.unwrap();
        expect(page.locator("dialog[open]")).to_contain_text("maya@example.test").await.unwrap();
        page.keyboard().press("Escape",None).await.unwrap();
        expect(page.locator("button[data-dialog-open=cancel-invitation-87801]")).to_be_focused().await.unwrap();
        page.get_by_label("Email",true).fill("maya@example.test",None).await.unwrap();
        page.get_by_role(AriaRole::Button,named("Send invitation")).press("Enter",None).await.unwrap();
        expect(page.get_by_label("Email",true)).to_have_attribute("aria-invalid","true").await.unwrap();
        expect(page.locator("#email")).to_be_focused().await.unwrap();
        expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
        capture(page,"error-320").await;
        page.get_by_role(AriaRole::Button,named("Cancel maya@example.test")).press("Enter",None).await.unwrap();
        page.get_by_role(AriaRole::Button,named("Cancel invitation maya@example.test")).press("Enter",None).await.unwrap();
        expect(page.locator(".admin-invitations")).to_contain_text("No invitations are available.").await.unwrap();
        capture(page,"empty-320").await;
        let remaining=journey.admin.query_one_raw(sea_orm::Statement::from_string(sea_orm::DbBackend::Postgres,"SELECT count(*) AS count FROM invitation_mail_attempts WHERE original_invitation_id=87801 AND invitation_id IS NULL")).await.unwrap().unwrap();
        assert_eq!(remaining.try_get::<i64>("","count").unwrap(),2);
    })).await;
}
