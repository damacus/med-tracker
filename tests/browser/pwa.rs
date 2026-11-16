use playwright_rs::{
    expect,
    protocol::{ScreenshotOptions, Viewport},
};
use serde_json::{Value, json};

use crate::harness;

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn pwa_shell_keeps_private_pages_network_only() {
    harness::journey(Viewport { width: 390, height: 844 }, |journey| {
        Box::pin(async move {
            let page = &journey.page;
            page.goto(&format!("{}/login", journey.origin), None).await.unwrap();
            let login: Value = page.evaluate(r#"async () => {
                const link = document.querySelector('link[rel=manifest]');
                const response = await fetch(link.href);
                const manifest = await response.json();
                await navigator.serviceWorker.ready;
                return {type: response.headers.get('content-type'), manifest};
            }"#, None::<&()>).await.unwrap();
            assert_eq!(login["manifest"]["start_url"], "/");
            assert_eq!(login["manifest"]["display"], "standalone");
            assert!(login["type"].as_str().unwrap().contains("application/manifest+json"));
            harness::sign_in(journey).await;
            assert_eq!(page.locator("link[rel=manifest]").count().await.unwrap(), 1);
            let cache_entries: Value = page.evaluate(r#"async () => Promise.all((await caches.keys()).map(async key => (await (await caches.open(key)).keys()).map(request => request.url)))"#, None::<&()>).await.unwrap();
            assert_eq!(cache_entries, json!([]));
            let context = page.context().unwrap();
            context.set_offline(true).await.unwrap();
            let directory = std::path::Path::new("docs/screenshots/pwa");
            std::fs::create_dir_all(directory).unwrap();
            for (name, viewport) in [
                ("offline-mobile", Viewport { width: 390, height: 844 }),
                ("offline-desktop", Viewport { width: 1440, height: 1000 }),
            ] {
                page.set_viewport_size(viewport).await.unwrap();
                page.goto(&format!("{}/", journey.origin), None).await.unwrap();
                let offline: String = page.evaluate("() => document.body.innerText", None::<&()>).await.unwrap();
                assert!(offline.contains("Connect to the internet to use MedTracker."), "{name}: {offline}");
                assert!(!offline.contains("Synthetic household"), "{name}: {offline}");
                assert!(!page.evaluate::<(), bool>("() => document.documentElement.scrollWidth > innerWidth", None).await.unwrap(), "{name}: horizontal overflow");
                page.screenshot_to_file(&directory.join(format!("{name}.png")), ScreenshotOptions::builder().full_page(true).build()).await.unwrap();
            }
            context.set_offline(false).await.unwrap();
            let private_url = format!("{}/households/persistence-fixture/medications", journey.origin);
            page.goto(&private_url, None).await.unwrap();
            expect(page.locator("header [data-app-profile]:visible")).to_be_visible().await.unwrap();
            let online: Value = page.evaluate(r#"async () => {
                const response = await fetch(location.href);
                return {status: response.status, cache: response.headers.get('cache-control'), body: await response.text()};
            }"#, None::<&()>).await.unwrap();
            assert_eq!(online["status"], 200);
            assert_eq!(online["cache"], "no-store");
            assert!(online["body"].as_str().unwrap().contains("Synthetic household"));
            page.locator("[data-app-profile]").click(None).await.unwrap();
            page.locator("[data-account-menu] form[action='/logout'] button").click(None).await.unwrap();
            expect(page.locator("form[action='/login']")).to_be_visible().await.unwrap();
            page.goto(&private_url, None).await.unwrap();
            expect(page.locator("form[action='/login']")).to_be_visible().await.unwrap();
            let signed_out: Value = page.evaluate(r#"async () => ({
                body: document.body.innerText,
                entries: await Promise.all((await caches.keys()).map(async key => (await (await caches.open(key)).keys()).map(request => request.url)))
            })"#, None::<&()>).await.unwrap();
            assert!(!signed_out["body"].as_str().unwrap().contains("Synthetic household"));
            assert_eq!(signed_out["entries"], json!([]));
        })
    }).await;
}
