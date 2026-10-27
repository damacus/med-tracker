use crate::harness::{self, named};
use playwright_rs::expect;
use playwright_rs::protocol::{AriaRole, BrowserContextOptions};
use playwright_rs::protocol::{ScreenshotOptions, Viewport};
use sea_orm::ConnectionTrait;

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn shared_layout_favicon_loads_on_login_and_dashboard() {
    harness::journey(
        Viewport {
            width: 1440,
            height: 1000,
        },
        |journey| {
            Box::pin(async move {
                let page = &journey.page;
                for screen in ["login", "dashboard"] {
                    if screen == "dashboard" {
                        harness::sign_in(journey).await;
                    }
                    for (name, width, height) in [("desktop", 1440, 1000), ("mobile", 390, 844)] {
                        page.set_viewport_size(Viewport { width, height })
                            .await
                            .unwrap();
                        let path = if screen == "login" {
                            "/login"
                        } else {
                            "/households/persistence-fixture/dashboard"
                        };
                        page.goto(&format!("{}{path}", journey.origin), None)
                            .await
                            .unwrap();
                        let icon = page.evaluate::<(), serde_json::Value>(r#"async () => {
                        const link = document.querySelector('link[rel="icon"]');
                        if (!link) return null;
                        const response = await fetch(link.href);
                        const image = new Image();
                        image.src = link.href;
                        await image.decode();
                        return {href: new URL(link.href).pathname, type: link.type,
                            status: response.status, contentType: response.headers.get('content-type'),
                            width: image.naturalWidth, height: image.naturalHeight};
                    }"#, None).await.unwrap();
                        assert_eq!(icon, serde_json::json!({
                        "href": "/favicon.ico", "type": "image/png", "status": 200,
                        "contentType": "image/png", "width": 512, "height": 512
                    }), "{screen} at {width}px");
                        capture(page, &format!("favicon-{screen}-{name}")).await;
                    }
                }
            })
        },
    )
    .await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn shared_layout_desktop_navigation_survives_route_changes() {
    harness::journey(
        Viewport {
            width: 1440,
            height: 1000,
        },
        |journey| {
            Box::pin(async move {
                let page = &journey.page;
                harness::sign_in(journey).await;
                for (path, section) in [
                    ("dashboard", "dashboard"),
                    ("medications", "inventory"),
                    ("people", "people"),
                    ("locations", "locations"),
                    ("profile", "profile"),
                    ("reports", "reports"),
                    ("admin", "administration"),
                    ("medications/new", "inventory"),
                    ("medications/finder", "finder"),
                ] {
                    if path.contains('/') {
                        page.goto(
                            &format!("{}/households/persistence-fixture/{path}", journey.origin),
                            None,
                        )
                        .await
                        .unwrap();
                    } else {
                        page.locator(format!(".app-sidebar [data-section={section}]"))
                            .click(None)
                            .await
                            .unwrap();
                    }
                    expect(page.locator(".app-sidebar"))
                        .to_be_visible()
                        .await
                        .unwrap();
                    expect(page.locator(format!(
                        ".app-sidebar [data-section={section}][aria-current=page]"
                    )))
                    .to_be_visible()
                    .await
                    .unwrap();
                    assert_eq!(
                        page.locator(".app-sidebar [aria-current=page]")
                            .count()
                            .await
                            .unwrap(),
                        1
                    );
                    let switcher =
                        page.locator(".app-sidebar a[href=\"/households/choose?switch=1\"]");
                    expect(switcher.clone())
                        .to_contain_text("Synthetic household")
                        .await
                        .unwrap();
                    expect(
                        page.get_by_role(AriaRole::Link, named("Households Synthetic household")),
                    )
                    .to_be_visible()
                    .await
                    .unwrap();
                    if ["dashboard", "medications", "profile", "reports"].contains(&path) {
                        if path == "reports" {
                            expect(page.locator("body[data-navigation-ready]"))
                                .to_be_visible()
                                .await
                                .unwrap();
                        }
                        capture(page, &format!("{section}-desktop")).await;
                    }
                }
            })
        },
    )
    .await;
}

async fn capture(page: &playwright_rs::protocol::Page, name: &str) {
    let directory = std::path::Path::new("docs/screenshots/shared-layout");
    std::fs::create_dir_all(directory).unwrap();
    page.screenshot_to_file(
        &directory.join(format!("{name}.png")),
        ScreenshotOptions::default(),
    )
    .await
    .unwrap();
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn shared_layout_mobile_drawer_and_narrow_pages() {
    harness::journey(
        Viewport {
            width: 390,
            height: 844,
        },
        |journey| {
            Box::pin(async move {
                let page = &journey.page;
                harness::sign_in(journey).await;
                for width in [390, 320] {
                    page.set_viewport_size(Viewport { width, height: 844 })
                        .await
                        .unwrap();
                    for (path, section) in [
                        ("medications", "inventory"),
                        ("people", "people"),
                        ("profile", "profile"),
                        ("reports", "reports"),
                        ("add-medication", "inventory"),
                    ] {
                        page.goto(
                            &format!("{}/households/persistence-fixture/{path}", journey.origin),
                            None,
                        )
                        .await
                        .unwrap();
                        let trigger = page.get_by_role(AriaRole::Button, named("Open menu"));
                        trigger.click(None).await.unwrap();
                        expect(page.locator("#app-drawer"))
                            .to_be_visible()
                            .await
                            .unwrap();
                        expect(page.locator(format!(
                            "#app-drawer [data-section={section}][aria-current=page]"
                        )))
                        .to_be_visible()
                        .await
                        .unwrap();
                        page.keyboard().press("Escape", None).await.unwrap();
                        expect(page.locator("[data-app-drawer]")).to_be_focused().await.unwrap();
                    let overflowing = page.evaluate::<(), serde_json::Value>("() => [...document.querySelectorAll(\"main *\")].filter(e => e.getBoundingClientRect().right > innerWidth).filter(e => !e.closest(\".overflow-x-auto\")).map(e => ({tag:e.tagName,classes:e.className,text:e.textContent.slice(0,80),right:e.getBoundingClientRect().right}))", None).await.unwrap();
                    assert!(!page.evaluate::<(),bool>("() => document.documentElement.scrollWidth > innerWidth",None).await.unwrap(), "{path} at {width}: {overflowing}");
                    assert_eq!(
                            page.locator(".dashboard-mobile-rail")
                                .count()
                                .await
                                .unwrap(),
                            0
                        );
                        assert_eq!(
                            page.locator("[data-mobile-shortcuts]")
                                .count()
                                .await
                                .unwrap(),
                            1
                        );
                        if path == "add-medication" {
                            expect(page.locator("[data-mobile-shortcuts] a[href$=\"/medications\"][aria-current=page]")).to_be_visible().await.unwrap();
                        }
                        let screenshot = if path == "add-medication" { path } else { section };
                        if path == "reports" {
                            expect(page.locator("body[data-navigation-ready]"))
                                .to_be_visible()
                                .await
                                .unwrap();
                        }
                        capture(page, &format!("{screenshot}-{width}")).await;
                    }
                }
            })
        },
    )
    .await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn shared_layout_fallback_and_permission_boundaries() {
    harness::journey(Viewport {width:1440,height:1000}, |journey| {
        Box::pin(async move {
            let page=&journey.page;
            page.goto(&format!("{}/login",journey.origin),None).await.unwrap();
            assert_eq!(page.locator(".app-sidebar").count().await.unwrap(),0);
            capture(page,"login-desktop").await;
            harness::sign_in(journey).await;
            let context=page.context().unwrap();
            let no_script=context.browser().unwrap().new_context_with_options(BrowserContextOptions::builder().javascript_enabled(false).viewport(Viewport {width:390,height:844}).build()).await.unwrap();
            no_script.add_cookies(&context.cookies(None).await.unwrap()).await.unwrap();
            let fallback=no_script.new_page().await.unwrap();
            fallback.goto(&format!("{}/households/persistence-fixture/medications",journey.origin),None).await.unwrap();
            expect(fallback.locator(".app-sidebar")).to_be_visible().await.unwrap();
            expect(fallback.locator(".app-sidebar a[href$='/dashboard']")).to_be_visible().await.unwrap();
            expect(fallback.locator(".app-sidebar a[href$='/profile']")).to_be_visible().await.unwrap();
            expect(fallback.locator(".app-sidebar form[action='/logout'] button")).to_be_visible().await.unwrap();
            expect(fallback.get_by_role(AriaRole::Button,named("Appearance"))).to_be_visible().await.unwrap();
            capture(&fallback,"inventory-no-script-390").await;
            no_script.close().await.unwrap();
            page.route("**/profile/navigation",|route|async move {route.abort(None).await}).await.unwrap();
            page.reload(None).await.unwrap();
            expect(page.locator(".app-sidebar a[href$='/dashboard']")).to_be_visible().await.unwrap();
            expect(page.locator(".app-sidebar form[action='/logout'] button")).to_be_visible().await.unwrap();
            page.unroute("**/profile/navigation").await.unwrap();
            journey.admin.execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001").await.unwrap();
            page.reload(None).await.unwrap();
            expect(page.locator("[data-sidebar-account]")).to_be_visible().await.unwrap();
            assert_eq!(page.locator(".app-sidebar [data-section=administration]").count().await.unwrap(),0);
            journey.admin.execute_unprepared("UPDATE household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE id=74001").await.unwrap();
            page.goto(&format!("{}/households/choose?switch=1",journey.origin),None).await.unwrap();
            assert_eq!(page.locator(".app-sidebar").count().await.unwrap(),0);
            capture(page,"householdless-desktop").await;
            page.goto(&format!("{}/households/persistence-fixture/medications",journey.origin),None).await.unwrap();
            assert_eq!(page.locator(".app-sidebar").count().await.unwrap(),0);
        })
    }).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn shared_layout_platform_and_account_frames_do_not_invent_household_navigation() {
    harness::journey(Viewport {width:1440,height:1000}, |journey| {
        Box::pin(async move {
            let page=&journey.page;
            harness::sign_in(journey).await;
            journey.admin.execute_unprepared("INSERT INTO platform_admins (account_id,status,created_at,updated_at) VALUES (71001,'active',now(),now())").await.unwrap();
            page.reload(None).await.unwrap();
            page.locator("[data-sidebar-account]").click(None).await.unwrap();
            expect(page.locator("#app-account a[href='/platform/users']")).to_be_visible().await.unwrap();
            page.locator("#app-account a[href='/platform/users']").click(None).await.unwrap();
            expect(page.locator(".platform-nav")).to_be_visible().await.unwrap();
            assert_eq!(page.locator(".app-sidebar").count().await.unwrap(),0);
            capture(page,"platform-desktop").await;
            page.goto(&format!("{}/account/security",journey.origin),None).await.unwrap();
            assert_eq!(page.locator(".app-sidebar").count().await.unwrap(),0);
            capture(page,"account-security-desktop").await;
            page.set_viewport_size(Viewport {width:320,height:844}).await.unwrap();
            assert!(!page.evaluate::<(),bool>("() => document.documentElement.scrollWidth > innerWidth",None).await.unwrap());
            capture(page,"account-security-320").await;
        })
    }).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn shared_layout_search_and_account_do_not_compete() {
    harness::journey(
        Viewport {
            width: 1440,
            height: 1000,
        },
        |journey| {
            Box::pin(async move {
                let page = &journey.page;
                harness::sign_in(journey).await;
                page.locator("[data-sidebar-search]")
                    .click(None)
                    .await
                    .unwrap();
                expect(page.locator("#global_search_panel"))
                    .to_be_visible()
                    .await
                    .unwrap();
                page.keyboard().press("Escape", None).await.unwrap();
                expect(page.locator("[data-sidebar-search]"))
                    .to_be_focused()
                    .await
                    .unwrap();
                page.locator("[data-sidebar-account]")
                    .click(None)
                    .await
                    .unwrap();
                page.keyboard().press("Control+k", None).await.unwrap();
                expect(page.locator("#global_search_panel"))
                    .to_be_visible()
                    .await
                    .unwrap();
                assert_eq!(
                    page.locator(".app-navigation-panel[open]")
                        .count()
                        .await
                        .unwrap(),
                    0
                );
                page.keyboard().press("Escape", None).await.unwrap();
                expect(page.locator("[data-sidebar-account]"))
                    .to_be_focused()
                    .await
                    .unwrap();
            })
        },
    )
    .await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn shared_layout_search_handles_rapid_escape_and_queued_dialog_close() {
    harness::journey(Viewport { width:1440,height:1000 },|journey|Box::pin(async move {
        let page=&journey.page;
        harness::sign_in(journey).await;
        expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
        let observed=page.evaluate::<(),serde_json::Value>(r#"async () => {
            const panel=document.querySelector('[data-global-search]');
            const input=panel.querySelector('#global_search_query');
            const search=document.querySelector('[data-sidebar-search]');
            const account=document.querySelector('[data-sidebar-account]');
            const dialog=document.querySelector('#app-account');
            const focus=[];
            document.addEventListener('focusin',event=>focus.push(event.target.id || event.target.getAttribute('data-sidebar-search')!==null && 'sidebar-search' || event.target.getAttribute('data-sidebar-account')!==null && 'sidebar-account' || event.target.tagName));
            search.focus();
            search.click();
            const rapidBefore=document.activeElement===input?'input':'opener';
            document.activeElement.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true,cancelable:true}));
            const rapid={closed:panel.hidden,restored:document.activeElement===search,before:rapidBefore};
            panel.querySelector('[data-global-search-close]').click();
            account.click();
            const closed=new Promise(resolve=>dialog.addEventListener('close',resolve,{once:true}));
            document.activeElement.dispatchEvent(new KeyboardEvent('keydown',{key:'k',ctrlKey:true,bubbles:true,cancelable:true}));
            input.focus();
            await closed;
            const queued={closed:panel.hidden,inputFocused:document.activeElement===input,dialogClosed:!dialog.open};
            return {rapid,queued,focus};
        }"#,None).await.unwrap();
        assert_eq!(observed["rapid"]["closed"],true,"{observed}");
        assert_eq!(observed["rapid"]["restored"],true,"{observed}");
        assert_eq!(observed["queued"]["closed"],false,"{observed}");
        assert_eq!(observed["queued"]["inputFocused"],true,"{observed}");
        assert_eq!(observed["queued"]["dialogClosed"],true,"{observed}");
        page.keyboard().press("Escape",None).await.unwrap();
        expect(page.locator("[data-sidebar-account]")).to_be_focused().await.unwrap();
    })).await;
}
