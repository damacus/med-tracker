use crate::harness::{self, named};
use med_tracker::models::entities::{membership, person, security_audit_event};
use playwright_rs::expect;
use playwright_rs::protocol::{Animations, AriaRole, Page, ScreenshotOptions, Viewport};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};

async fn add_members(admin: &sea_orm::DatabaseConnection, range: std::ops::RangeInclusive<i64>) {
    for n in range {
        let now = chrono::Utc::now().naive_utc();
        let name = match n {
            1 => "Bulk Alpha".to_owned(),
            2 => "Bulk Beta".to_owned(),
            _ => format!("Bulk Member {n}"),
        };
        person::ActiveModel {
            id: Set(73500 + n),
            name: Set(name),
            person_type: Set(0),
            has_capacity: Set(true),
            portable_id: Set(uuid::Uuid::new_v4().to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(admin)
        .await
        .unwrap();
        membership::ActiveModel {
            id: Set(74500 + n),
            household_id: Set(72001),
            person_id: Set(73500 + n),
            role: Set("member".into()),
            status: Set(if n == 2 { "suspended" } else { "active" }.into()),
            joined_at: Set(now),
            created_at: Set(now),
            updated_at: Set(now),
            permissions_version: Set(1),
            ..Default::default()
        }
        .insert(admin)
        .await
        .unwrap();
    }
}
async fn cancellation_state(admin: &sea_orm::DatabaseConnection) -> (Vec<membership::Model>, u64) {
    let states = membership::Entity::find()
        .filter(membership::Column::Id.is_in([74501, 74502]))
        .order_by_asc(membership::Column::Id)
        .all(admin)
        .await
        .unwrap();
    let audits = security_audit_event::Entity::find()
        .filter(security_audit_event::Column::HouseholdId.eq(72001))
        .filter(security_audit_event::Column::EventType.eq("household_access.membership_changed"))
        .count(admin)
        .await
        .unwrap();
    (states, audits)
}

async fn capture(page: &Page, name: &str) {
    if page.locator("body.app-household").count().await.unwrap() > 0 {
        expect(page.locator("body[data-navigation-ready]"))
            .to_be_visible()
            .await
            .unwrap();
    }
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
        membership::Entity::update_many().col_expr(membership::Column::Role, Expr::value("owner"))
            .filter(membership::Column::Id.eq(74001)).exec(&journey.admin).await.unwrap();
        add_members(&journey.admin, 1..=3).await;
        harness::sign_in(journey).await;
        let page=&journey.page;
        let listing=format!("{}/households/persistence-fixture/admin/users?q=Bulk&lang=en",journey.origin);
        page.goto(&listing,None).await.unwrap();
        for width in [1440,390,320] {
            page.set_viewport_size(Viewport{width,height:if width==1440{1000}else{844}}).await.unwrap();
            capture(page,&format!("selection-{width}")).await;
            assert!(page.evaluate::<(),bool>("() => getComputedStyle(document.querySelector('thead')).display !== 'none' && [...document.querySelectorAll('thead th')].every(th=>th.scope==='col') && document.querySelector('td[headers]').getAttribute('headers').includes('member-row-')",None).await.unwrap());
        }
        add_members(&journey.admin, 4..=27).await;
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
        let before_cancel = cancellation_state(&journey.admin).await;
        page.get_by_role(AriaRole::Button,named("Suspend selected")).press("Enter",None).await.unwrap();
        expect(page.get_by_role(AriaRole::Button,named("Confirm Suspend"))).to_be_visible().await.unwrap();
        page.get_by_role(AriaRole::Link,named("Back to members")).press("Enter",None).await.unwrap();
        expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
        for context in ["q=Bulk","role=all","status=all","sort=name","direction=asc","page=1","lang=en"] {
            assert!(page.url().contains(context),"missing returned context {context}: {}",page.url());
        }
        assert_eq!(page.evaluate::<(),i64>("() => document.querySelectorAll('input[name=membership_ids]:checked').length",None).await.unwrap(),0);
        let after_cancel = cancellation_state(&journey.admin).await;
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
        harness::device_appearance(page, "dark").await;
        capture(page,"selection-dark-1440").await;
        harness::device_appearance(page, "light").await;
        page.set_viewport_size(Viewport{width:320,height:844}).await.unwrap();
        page.get_by_role(AriaRole::Button,named("Revoke selected")).press("Enter",None).await.unwrap();
        expect(page.locator("main")).to_contain_text("Select between 1 and 25").await.unwrap();
        capture(page,"error-320").await;
        page.goto(&format!("{}/households/persistence-fixture/admin/users?q=No-match&lang=en",journey.origin),None).await.unwrap();
        capture(page,"empty-320").await;
    })).await;
}
