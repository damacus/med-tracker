use crate::harness::{self, named};
use playwright_rs::expect;
use playwright_rs::protocol::{Animations, ScreenshotOptions, Viewport};
use playwright_rs::protocol::{AriaRole, BrowserContextOptions};
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
                    let switcher = page.locator(".app-sidebar [data-household-trigger]");
                    expect(switcher.clone())
                        .to_contain_text("Synthetic household")
                        .await
                        .unwrap();
                    expect(switcher.clone()).to_be_visible().await.unwrap();
                    expect(switcher)
                        .to_have_attribute("title", "Households")
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
        ScreenshotOptions::builder()
            .animations(Animations::Disabled)
            .build(),
    )
    .await
    .unwrap();
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn shared_layout_account_menu_signs_out_from_desktop_and_mobile() {
    for (name, width, height, scripts) in [
        ("desktop", 1440, 1000, true),
        ("mobile", 390, 844, true),
        ("desktop-no-script", 1440, 1000, false),
        ("mobile-no-script", 390, 844, false),
    ] {
        harness::journey(Viewport { width, height }, |journey| {
            Box::pin(async move {
                harness::sign_in(journey).await;
                let original = journey.page.context().unwrap();
                let isolated = original
                    .browser()
                    .unwrap()
                    .new_context_with_options(
                        BrowserContextOptions::builder()
                            .javascript_enabled(scripts)
                            .viewport(Viewport { width, height })
                            .build(),
                    )
                    .await
                    .unwrap();
                isolated
                    .add_cookies(&original.cookies(None).await.unwrap())
                    .await
                    .unwrap();
                let account_page = isolated.new_page().await.unwrap();
                let page = &account_page;
                page.goto(
                    &format!(
                        "{}/households/persistence-fixture/medications",
                        journey.origin
                    ),
                    None,
                )
                .await
                .unwrap();
                page.locator("[data-app-profile]").focus().await.unwrap();
                page.keyboard().press("Enter", None).await.unwrap();
                expect(page.locator("[data-account-menu] a[href$='/dashboard']"))
                    .to_be_visible()
                    .await
                    .unwrap();
                expect(page.locator("[data-account-menu] a[href$='/profile']"))
                    .to_be_visible()
                    .await
                    .unwrap();
                expect(page.locator("[data-account-menu] a[href$='/admin']"))
                    .to_be_visible()
                    .await
                    .unwrap();
                expect(page.locator("[data-account-menu] form[action='/logout'] button"))
                    .to_be_visible()
                    .await
                    .unwrap();
                assert!(page.evaluate::<(), bool>("() => { const menu = document.querySelector('.app-account-options'); const rect = menu.getBoundingClientRect(); return getComputedStyle(menu).position === 'absolute' && rect.left >= 0 && rect.right <= innerWidth && rect.bottom <= innerHeight && document.querySelector('.shell-header').getBoundingClientRect().height < 100; }", None).await.unwrap());
                capture(page, &format!("account-menu-{name}")).await;
                if scripts {
                    page.keyboard().press("Escape", None).await.unwrap();
                } else {
                    page.locator("[data-app-profile]")
                        .click(None)
                        .await
                        .unwrap();
                }
                expect(page.locator("[data-app-profile]"))
                    .to_be_focused()
                    .await
                    .unwrap();
                expect(page.locator("[data-account-menu] form[action='/logout'] button"))
                    .to_be_hidden()
                    .await
                    .unwrap();
                page.locator("[data-app-profile]")
                    .click(None)
                    .await
                    .unwrap();
                page.locator("[data-account-menu] a[href$='/profile']")
                    .click(None)
                    .await
                    .unwrap();
                assert!(page.url().ends_with("/profile"));
                page.locator("[data-app-profile]")
                    .click(None)
                    .await
                    .unwrap();
                page.locator("[data-account-menu] form[action='/logout'] button")
                    .click(None)
                    .await
                    .unwrap();
                expect(page.locator("form[action='/login']"))
                    .to_be_visible()
                    .await
                    .unwrap();
                page.goto(
                    &format!(
                        "{}/households/persistence-fixture/medications",
                        journey.origin
                    ),
                    None,
                )
                .await
                .unwrap();
                expect(page.locator("form[action='/login']"))
                    .to_be_visible()
                    .await
                    .unwrap();
                isolated.close().await.unwrap();
            })
        })
        .await;
    }
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
                        let trigger = page.get_by_label("Open menu", true);
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
            fallback.locator("[data-app-drawer]").click(None).await.unwrap();
            expect(fallback.locator("#app-drawer a[href$='/dashboard']")).to_be_visible().await.unwrap();
            expect(fallback.locator("#app-drawer a[href$='/profile']")).to_be_visible().await.unwrap();
            expect(fallback.locator("#app-drawer form[action='/logout'] button")).to_be_visible().await.unwrap();
            fallback.locator("#app-drawer [data-household-trigger]").click(None).await.unwrap();
            expect(fallback.locator("#drawer-households button[aria-current=true]")).to_be_visible().await.unwrap();
            fallback.locator("#drawer-households button[aria-current=true]").click(None).await.unwrap();
            assert!(fallback.url().ends_with("/dashboard"));
            fallback.locator("[data-app-profile]").click(None).await.unwrap();
            expect(fallback.locator("[data-account-menu] form[action='/logout'] button")).to_be_visible().await.unwrap();
            fallback.locator("[data-account-section=profile]").click(None).await.unwrap();
            expect(fallback.locator("#security form[action='/logout'] button")).to_be_visible().await.unwrap();
            expect(fallback.locator("[data-profile-navigation-fallback]")).to_have_count(0).await.unwrap();
            assert_eq!(fallback.locator("[data-dialog-open='appearance-dialog']").count().await.unwrap(), 0);
            capture(&fallback,"inventory-no-script-390").await;
            fallback.goto(&format!("{}/households/persistence-fixture/dashboard",journey.origin),None).await.unwrap();
            fallback.locator("[data-dashboard-selector] > summary").click(None).await.unwrap();
            fallback.locator("[data-person-id=all]").click(None).await.unwrap();
            expect(fallback.locator("[data-dashboard-selector]")).to_have_attribute("data-selected-person", "all").await.unwrap();
            expect(fallback.locator("[data-dashboard-selector] button")).to_have_count(0).await.unwrap();
            no_script.close().await.unwrap();
            page.route("**/profile/navigation",|route|async move {route.abort(None).await}).await.unwrap();
            page.route("**/static/js/app-navigation.js",|route|async move {route.abort(None).await}).await.unwrap();
            page.reload(None).await.unwrap();
            expect(page.locator(".app-sidebar [data-section=dashboard]")).to_be_visible().await.unwrap();
            page.locator("[data-app-profile]").click(None).await.unwrap();
            page.locator("[data-account-section=profile]").click(None).await.unwrap();
            page.locator("#profile-tab-security").click(None).await.unwrap();
            expect(page.locator("#security form[action='/logout'] button")).to_be_visible().await.unwrap();
            page.unroute("**/static/js/app-navigation.js").await.unwrap();
            page.unroute("**/profile/navigation").await.unwrap();
            journey.admin.execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001").await.unwrap();
            page.reload(None).await.unwrap();
            expect(page.locator("[data-app-profile]")).to_be_visible().await.unwrap();
            expect(page.locator("[data-account-section=administration]")).to_have_count(0).await.unwrap();
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
async fn shared_layout_server_navigation_and_rail_persist_across_household_pages() {
    harness::journey(Viewport { width: 1440, height: 1000 }, |journey| Box::pin(async move {
        harness::sign_in(journey).await;
        let page = &journey.page;
        page.locator("label[for=app-sidebar-expanded]").click(None).await.unwrap();
        for path in ["dashboard", "medications", "people", "locations", "reports", "profile", "admin"] {
            page.goto(&format!("{}/households/persistence-fixture/{path}", journey.origin), None).await.unwrap();
            assert!(page.evaluate::<(), bool>("() => { const toggle = document.querySelector('[data-sidebar-expanded]'); return !toggle.checked && !toggle.hasAttribute('checked') && document.querySelector('.app-sidebar').getBoundingClientRect().width === 80 && !performance.getEntriesByType('resource').some(entry => new URL(entry.name).pathname.endsWith('/profile/navigation')); }", None).await.unwrap(), "{path}");
            expect(page.get_by_role(AriaRole::Link, named("MedTracker"))).to_be_visible().await.unwrap();
            assert!(page.evaluate::<(), bool>("() => getComputedStyle(document.querySelector('.app-sidebar-search kbd')).display === 'none'", None).await.unwrap(), "shortcut hint visible in {path} icon rail");
            expect(page.locator("[data-account-section=profile]")).to_have_attribute("href", "/households/persistence-fixture/profile").await.unwrap();
            page.locator(".app-sidebar [data-household-trigger]").click(None).await.unwrap();
            expect(page.locator("#sidebar-households button[aria-current=true]")).to_be_visible().await.unwrap();
            page.keyboard().press("Escape", None).await.unwrap();
        }
        capture(page, "profile-icon-rail").await;
        page.set_viewport_size(Viewport { width: 390, height: 844 }).await.unwrap();
        expect(page.locator("[data-sidebar-expanded]")).to_be_hidden().await.unwrap();
    })).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn shared_layout_direct_pages_persist_new_csrf_cookie_for_shell_forms() {
    harness::journey(
        Viewport {
            width: 1440,
            height: 1000,
        },
        |journey| {
            Box::pin(async move {
                harness::sign_in(journey).await;
                let original = journey.page.context().unwrap();
                let session_cookies = original
                    .cookies(None)
                    .await
                    .unwrap()
                    .into_iter()
                    .filter(|cookie| !cookie.name.contains("csrf"))
                    .collect::<Vec<_>>();
                for path in [
                    "search",
                    "medications/workflow",
                    "medications/80001/dosage_options",
                ] {
                    let isolated = original
                        .browser()
                        .unwrap()
                        .new_context_with_options(
                            BrowserContextOptions::builder()
                                .javascript_enabled(false)
                                .viewport(Viewport {
                                    width: 1440,
                                    height: 1000,
                                })
                                .build(),
                        )
                        .await
                        .unwrap();
                    isolated.add_cookies(&session_cookies).await.unwrap();
                    let page = isolated.new_page().await.unwrap();
                    page.goto(
                        &format!("{}/households/persistence-fixture/{path}", journey.origin),
                        None,
                    )
                    .await
                    .unwrap();
                    assert!(
                        isolated
                            .cookies(None)
                            .await
                            .unwrap()
                            .iter()
                            .any(|cookie| cookie.name.contains("csrf") && !cookie.value.is_empty()),
                        "{path}"
                    );
                    page.locator(".app-sidebar [data-household-trigger]")
                        .click(None)
                        .await
                        .unwrap();
                    page.locator(
                        "#sidebar-households form[action='/households/select'] button[aria-current=true]",
                    )
                    .click(None)
                    .await
                    .unwrap();
                    assert!(page.url().ends_with("/dashboard"), "{path}: {}", page.url());
                    isolated.close().await.unwrap();
                }
            })
        },
    )
    .await;
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
            page.locator("[data-app-profile]").click(None).await.unwrap();
            page.locator("[data-account-section=profile]").click(None).await.unwrap();
            page.locator("#profile-tab-security").click(None).await.unwrap();
            let platform_link=page.locator(".app-sidebar [data-sidebar-navigation] a[href='/platform/users']");
            expect(platform_link.clone()).to_be_visible().await.unwrap();
            platform_link.click(None).await.unwrap();
            expect(page.locator(".platform-nav")).to_be_visible().await.unwrap();
            assert_eq!(page.locator(".app-sidebar").count().await.unwrap(),0);
            capture(page,"platform-desktop").await;
            page.set_viewport_size(Viewport {width:390,height:844}).await.unwrap();
            page.goto(&format!("{}/admin/nhs-dmd",journey.origin),None).await.unwrap();
            expect(page.get_by_role(AriaRole::Link, named("Profile"))).to_be_visible().await.unwrap();
            page.set_viewport_size(Viewport {width:1440,height:1000}).await.unwrap();
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
                assert_eq!(
                    page.locator("#global_search_panel .modal-box [data-global-search-close]")
                        .count()
                        .await
                        .unwrap(),
                    0,
                );
                expect(page.locator("#global_search_query"))
                    .to_be_focused()
                    .await
                    .unwrap();
                for width in [1440, 390, 320] {
                    page.set_viewport_size(Viewport {
                        width,
                        height: 1000,
                    })
                    .await
                    .unwrap();
                    capture(page, &format!("search-spotlight-{width}")).await;
                }
                page.set_viewport_size(Viewport {
                    width: 1440,
                    height: 1000,
                })
                .await
                .unwrap();
                page.keyboard().press("Escape", None).await.unwrap();
                expect(page.locator("[data-sidebar-search]"))
                    .to_be_focused()
                    .await
                    .unwrap();
                page.locator(".app-sidebar [data-household-trigger]")
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
                expect(page.locator(".app-sidebar [data-household-trigger]"))
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
async fn shared_layout_mobile_native_dialog_transitions() {
    harness::journey(
        Viewport {
            width: 390,
            height: 844,
        },
        |journey| {
            Box::pin(async move {
                let page = &journey.page;
                harness::sign_in(journey).await;
                let trigger = page.locator("[data-app-drawer]");
                trigger.click(None).await.unwrap();
                expect(page.locator("#app-drawer:modal"))
                    .to_be_visible()
                    .await
                    .unwrap();
                page.locator("#app-drawer [data-household-trigger]")
                    .click(None)
                    .await
                    .unwrap();
                capture(page, "mobile-dialog-households").await;
                page.keyboard().press("Escape", None).await.unwrap();
                expect(page.locator("#app-drawer"))
                    .to_be_visible()
                    .await
                    .unwrap();
                page.keyboard().press("Control+k", None).await.unwrap();
                expect(page.locator("#global_search_panel"))
                    .to_be_visible()
                    .await
                    .unwrap();
                capture(page, "mobile-dialog-search").await;
                page.keyboard().press("Escape", None).await.unwrap();
                expect(trigger.clone()).to_be_focused().await.unwrap();
                trigger.click(None).await.unwrap();
                expect(page.locator("#app-drawer:modal"))
                    .to_be_visible()
                    .await
                    .unwrap();
                page.mouse().click(350.0, 400.0, None).await.unwrap();
                expect(page.locator("#app-drawer"))
                    .to_be_hidden()
                    .await
                    .unwrap();
                expect(trigger.clone()).to_be_focused().await.unwrap();
                trigger.click(None).await.unwrap();
                expect(page.locator("#app-drawer:modal"))
                    .to_be_visible()
                    .await
                    .unwrap();
                page.set_viewport_size(Viewport {
                    width: 1440,
                    height: 1000,
                })
                .await
                .unwrap();
                expect(page.locator("#app-drawer"))
                    .to_be_hidden()
                    .await
                    .unwrap();
                capture(page, "mobile-dialog-resized-desktop").await;
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
            const account=document.querySelector('.app-sidebar [data-household-trigger]');
            const dialog=document.querySelector('#sidebar-households');
            const focus=[];
            document.addEventListener('focusin',event=>focus.push(event.target.id || event.target.getAttribute('data-sidebar-search')!==null && 'sidebar-search' || event.target.getAttribute('data-household-trigger')!==null && 'sidebar-account' || event.target.tagName));
            search.focus();
            search.click();
            const rapidBefore=document.activeElement===input?'input':'opener';
            const queuedSearchClose=new Promise(resolve=>panel.addEventListener('close',resolve,{once:true}));
            panel.dispatchEvent(new Event('cancel',{cancelable:true}));
            const rapid={closed:!panel.open,restored:document.activeElement===search,before:rapidBefore};
            panel.querySelector('[data-global-search-close]').click();
            account.click();
            const closed=new Promise(resolve=>dialog.addEventListener('close',resolve,{once:true}));
            document.activeElement.dispatchEvent(new KeyboardEvent('keydown',{key:'k',ctrlKey:true,bubbles:true,cancelable:true}));
            input.focus();
            await closed;
            await queuedSearchClose;
            const queued={closed:!panel.open,inputFocused:document.activeElement===input,dialogClosed:!dialog.closest('details').open};
            return {rapid,queued,focus};
        }"#,None).await.unwrap();
        assert_eq!(observed["rapid"]["closed"],true,"{observed}");
        assert_eq!(observed["rapid"]["restored"],true,"{observed}");
        assert_eq!(observed["queued"]["closed"],false,"{observed}");
        assert_eq!(observed["queued"]["inputFocused"],true,"{observed}");
        assert_eq!(observed["queued"]["dialogClosed"],true,"{observed}");
        page.keyboard().press("Escape",None).await.unwrap();
        expect(page.locator(".app-sidebar [data-household-trigger]")).to_be_focused().await.unwrap();
    })).await;
}
