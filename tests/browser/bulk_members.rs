use crate::harness::{self, named};
use playwright_rs::expect;
use playwright_rs::protocol::{Animations, AriaRole, Page, ScreenshotOptions, Viewport};
use sea_orm::{ConnectionTrait, DbBackend, Statement};

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
    page.evaluate::<(), bool>("() => {scrollTo(0,0);return true}", None)
        .await
        .unwrap();
    let directory = std::path::Path::new("docs/screenshots/bulk-member-access");
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
async fn bulk_member_selection_confirmation_partial_retry_and_responsive_journey() {
    harness::journey(Viewport{width:1440,height:1000},|journey|Box::pin(async move {
        journey.admin.execute_unprepared("UPDATE household_memberships SET role='owner' WHERE id=74001; INSERT INTO people(id,name,person_type,has_capacity,created_at,updated_at) SELECT 73500+n,CASE n WHEN 1 THEN 'Bulk Alpha' WHEN 2 THEN 'Bulk Beta' ELSE 'Bulk Member '||n END,0,true,now(),now() FROM generate_series(1,3) n; INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) SELECT 74500+n,72001,73500+n,'member',CASE n WHEN 2 THEN 'suspended' ELSE 'active' END,now(),now(),now() FROM generate_series(1,3) n").await.unwrap();
        harness::sign_in(journey).await;
        let page=&journey.page;
        let listing=format!("{}/households/persistence-fixture/admin/users?q=Bulk&lang=en",journey.origin);
        page.goto(&listing,None).await.unwrap();
        for width in [1440,390,320] {
            page.set_viewport_size(Viewport{width,height:if width==1440{1000}else{844}}).await.unwrap();
            capture(page,&format!("selection-{width}")).await;
            assert!(page.evaluate::<(),bool>("() => getComputedStyle(document.querySelector('thead')).display !== 'none' && [...document.querySelectorAll('thead th')].every(th=>th.scope==='col') && document.querySelector('td[headers]').getAttribute('headers').includes('member-row-')",None).await.unwrap());
        }
        journey.admin.execute_unprepared("INSERT INTO people(id,name,person_type,has_capacity,created_at,updated_at) SELECT 73500+n,'Bulk Member '||n,0,true,now(),now() FROM generate_series(4,27) n; INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) SELECT 74500+n,72001,73500+n,'member','active',now(),now(),now() FROM generate_series(4,27) n").await.unwrap();
        page.goto(&listing,None).await.unwrap();
        expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
        page.get_by_label("Select everyone on this page",true).press("Space",None).await.unwrap();
        assert_eq!(page.evaluate::<(),i64>("() => document.querySelectorAll('input[name=membership_ids]:checked').length",None).await.unwrap(),25);
        page.get_by_role(AriaRole::Link,named("Next page")).press("Enter",None).await.unwrap();
        expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
        assert_eq!(page.evaluate::<(),i64>("() => document.querySelectorAll('input[name=membership_ids]:checked').length",None).await.unwrap(),0);
        page.goto(&listing,None).await.unwrap();
        expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
        page.get_by_label("Select Bulk Alpha",true).press("Space",None).await.unwrap();
        page.get_by_label("Name or email",true).fill("Bulk Beta",None).await.unwrap();
        page.get_by_role(AriaRole::Button,named("Apply filters")).press("Enter",None).await.unwrap();
        expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
        assert_eq!(page.evaluate::<(),i64>("() => document.querySelectorAll('input[name=membership_ids]:checked').length",None).await.unwrap(),0);
        page.goto(&listing,None).await.unwrap();
        expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
        page.get_by_label("Select Bulk Alpha",true).press("Space",None).await.unwrap();
        page.get_by_label("Select Bulk Beta",true).press("Space",None).await.unwrap();
        let unchanged_sql="SELECT jsonb_build_object('states',(SELECT jsonb_agg(jsonb_build_array(id,status,permissions_version,updated_at) ORDER BY id) FROM household_memberships WHERE id IN (74501,74502)),'audits',(SELECT count(*) FROM security_audit_events WHERE household_id=72001 AND event_type='household_access.membership_changed' AND metadata->>'target_membership_id' IN ('74501','74502'))) AS snapshot";
        let before_cancel:serde_json::Value=journey.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,unchanged_sql)).await.unwrap().unwrap().try_get("","snapshot").unwrap();
        page.get_by_role(AriaRole::Button,named("Suspend selected")).press("Enter",None).await.unwrap();
        expect(page.get_by_role(AriaRole::Button,named("Confirm Suspend"))).to_be_visible().await.unwrap();
        page.get_by_role(AriaRole::Link,named("Back to members")).press("Enter",None).await.unwrap();
        expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
        for context in ["q=Bulk","role=all","status=all","sort=name","direction=asc","page=1","lang=en"] {
            assert!(page.url().contains(context),"missing returned context {context}: {}",page.url());
        }
        assert_eq!(page.evaluate::<(),i64>("() => document.querySelectorAll('input[name=membership_ids]:checked').length",None).await.unwrap(),0);
        let after_cancel:serde_json::Value=journey.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,unchanged_sql)).await.unwrap().unwrap().try_get("","snapshot").unwrap();
        assert_eq!(after_cancel,before_cancel,"cancelling preview must not change member state or target audit");
        page.get_by_label("Select Bulk Alpha",true).press("Space",None).await.unwrap();
        page.get_by_label("Select Bulk Beta",true).press("Space",None).await.unwrap();
        page.get_by_role(AriaRole::Button,named("Suspend selected")).press("Enter",None).await.unwrap();
        expect(page.get_by_role(AriaRole::Button,named("Confirm Suspend"))).to_be_visible().await.unwrap();
        expect(page.locator("main")).to_contain_text("Bulk Alpha").await.unwrap();
        expect(page.locator("main")).to_contain_text("Bulk Beta").await.unwrap();
        expect(page.locator("main")).to_contain_text("Account sign-in is unchanged.").await.unwrap();
        capture(page,"confirmation-320").await;
        page.get_by_role(AriaRole::Button,named("Confirm Suspend")).press("Enter",None).await.unwrap();
        expect(page.locator("main")).to_contain_text("Household access changed.").await.unwrap();
        expect(page.locator("main")).to_contain_text("Membership state changed").await.unwrap();
        assert_eq!(page.evaluate::<(),i64>("() => document.querySelectorAll('input[name=membership_ids]:checked').length",None).await.unwrap(),1);
        capture(page,"results-320").await;
        page.get_by_role(AriaRole::Button,named("Review selected failures again")).press("Enter",None).await.unwrap();
        expect(page.locator("main")).to_contain_text("Bulk Beta").await.unwrap();
        assert!(!page.evaluate::<(),bool>("() => document.querySelector('main').textContent.includes('Bulk Alpha')",None).await.unwrap());
        page.goto(&format!("{}/households/persistence-fixture/admin/users?q=Bulk%20Beta&lang=en",journey.origin),None).await.unwrap();
        page.set_viewport_size(Viewport{width:1440,height:1000}).await.unwrap();
        expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
        page.locator(".app-sidebar [data-sidebar-account]").click(None).await.unwrap();
        page.get_by_role(AriaRole::Button,named("Appearance")).click(None).await.unwrap();
        page.locator("[data-appearance-choice=dark]").click(None).await.unwrap();
        page.locator("#appearance-dialog [data-dialog-close]").click(None).await.unwrap();
        capture(page,"selection-dark-1440").await;
        page.locator(".app-sidebar [data-sidebar-account]").click(None).await.unwrap();
        page.get_by_role(AriaRole::Button,named("Appearance")).click(None).await.unwrap();
        page.locator("[data-appearance-choice=system]").click(None).await.unwrap();
        page.locator("#appearance-dialog [data-dialog-close]").click(None).await.unwrap();
        page.set_viewport_size(Viewport{width:320,height:844}).await.unwrap();
        page.get_by_role(AriaRole::Button,named("Revoke selected")).press("Enter",None).await.unwrap();
        expect(page.locator("main")).to_contain_text("Select between 1 and 25").await.unwrap();
        capture(page,"error-320").await;
        page.goto(&format!("{}/households/persistence-fixture/admin/users?q=No-match&lang=en",journey.origin),None).await.unwrap();
        capture(page,"empty-320").await;
    })).await;
}
