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
            .unwrap()
    );
    page.evaluate::<(), bool>("() => { window.scrollTo(0,0); return true; }", None)
        .await
        .unwrap();
    let directory = std::path::Path::new("docs/screenshots/member-details");
    std::fs::create_dir_all(directory).unwrap();
    page.screenshot_to_file(
        &directory.join(format!("{name}.png")),
        ScreenshotOptions::builder()
            .animations(Animations::Disabled)
            .full_page(true)
            .build(),
    )
    .await
    .unwrap();
}
#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn member_detail_history_and_management_keyboard_journey() {
    harness::journey(Viewport{width:1440,height:1000},|journey|Box::pin(async move {
        journey.admin.execute_unprepared("UPDATE household_memberships SET role='owner' WHERE id=74001; INSERT INTO people(id,name,email,person_type,has_capacity,created_at,updated_at) VALUES(73016,'Detail dependant','detail@example.test',1,false,now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74016,72001,73016,'member','suspended',now(),now(),now()); INSERT INTO security_audit_events(household_id,actor_account_id,actor_membership_id,event_type,metadata,created_at,updated_at) VALUES(72001,71001,74001,'household_access.membership_changed',jsonb_build_object('target_membership_id',74016,'previous_state',jsonb_build_object('status','active'),'new_state',jsonb_build_object('status','suspended'),'outcome','success'),now(),now()),(72001,71001,74001,'household_access.membership_changed',jsonb_build_object('target_membership_id',74016,'outcome','rejected'),now(),now())").await.unwrap();
        harness::sign_in(journey).await;
        let page=&journey.page;
        page.goto(&format!("{}/households/persistence-fixture/admin/users?q=Detail&status=suspended&sort=email&direction=desc&page=1&lang=en",journey.origin),None).await.unwrap();
        capture(page,"directory-1440").await;
        page.get_by_role(AriaRole::Link,named("Detail dependant")).press("Enter",None).await.unwrap();
        expect(page.get_by_role(AriaRole::Heading,named("Detail dependant"))).to_be_visible().await.unwrap();
        expect(page.locator(".member-history")).to_contain_text("Rejected attempt").await.unwrap();
        for width in [1440,390,320] {
            page.set_viewport_size(Viewport{width,height:if width==1440{1000}else{844}}).await.unwrap();
            capture(page,&format!("detail-{width}")).await;
        }
        page.locator("summary[aria-label='Restore Detail dependant']").press("Enter",None).await.unwrap();
        expect(page.get_by_role(AriaRole::Button,named("Confirm Restore"))).to_be_visible().await.unwrap();
        page.get_by_role(AriaRole::Button,named("Confirm Restore")).press("Enter",None).await.unwrap();
        expect(page.locator("section > .alert[role=status]")).to_contain_text("Restored").await.unwrap();
        assert!(page.url().contains("/admin/memberships/74016?"));
        assert!(page.url().contains("q=Detail"));
        assert!(page.url().contains("lang=en"));
        page.get_by_role(AriaRole::Link,named("Edit household role for Detail dependant")).press("Enter",None).await.unwrap();
        expect(page.get_by_label("Household role",true)).to_be_visible().await.unwrap();
        page.get_by_role(AriaRole::Link,named("Cancel")).press("Enter",None).await.unwrap();
        expect(page.get_by_role(AriaRole::Heading,named("Detail dependant"))).to_be_visible().await.unwrap();
        page.set_viewport_size(Viewport{width:1440,height:1000}).await.unwrap();
        harness::device_appearance(page, "dark").await;
        capture(page,"detail-dark-1440").await;
        harness::device_appearance(page, "light").await;
        page.goto(&format!("{}/households/persistence-fixture/admin/memberships/74980",journey.origin),None).await.unwrap();
        page.set_viewport_size(Viewport{width:320,height:844}).await.unwrap();
        expect(page.locator(".member-history")).to_contain_text("No recorded membership activity.").await.unwrap();
        capture(page,"empty-320").await;
        page.goto(&format!("{}/households/persistence-fixture/admin/memberships/74001?lang=en",journey.origin),None).await.unwrap();
        page.locator("summary[aria-label='Suspend persistence@example.test']").press("Enter",None).await.unwrap();
        page.get_by_role(AriaRole::Button,named("Confirm Suspend")).press("Enter",None).await.unwrap();
        expect(page.locator("main")).to_contain_text("Last active owner cannot be removed").await.unwrap();
        capture(page,"error-320").await;
    })).await;
}
