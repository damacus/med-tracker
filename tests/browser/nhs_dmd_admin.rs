use crate::harness;
use playwright_rs::expect;
use playwright_rs::protocol::{Animations, Page, ScreenshotOptions, Viewport};
use sea_orm::ConnectionTrait;

async fn capture(page: &Page, name: &str) {
    let directory = std::path::Path::new("docs/screenshots/browser-journeys");
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
async fn nhs_dmd_failed_import_shows_trace_reference() {
    harness::journey(Viewport {width:1440,height:1000}, |journey| Box::pin(async move {
        let page=&journey.page;
        journey.admin.execute_unprepared("INSERT INTO platform_admins(account_id,status,created_at,updated_at) VALUES(71001,'active',now(),now()); INSERT INTO nhs_dmd_imports(id,uploaded_filename,status,error_message,trace_id,total_records,processed_records,imported_count,created_count,updated_count,unchanged_count,skipped_count,skipped_expired_count,skipped_invalid_count,skipped_missing_name_count,created_at,updated_at) VALUES(92001,'release.zip',5,'The import stopped making progress. Upload the release again.','0123456789abcdef0123456789abcdef',0,0,0,0,0,0,0,0,0,0,now(),now())").await.unwrap();
        harness::sign_in(journey).await;
        page.goto(&format!("{}/admin/nhs-dmd",journey.origin),None).await.unwrap();

        expect(page.locator(".alert-error")).to_contain_text("The import stopped making progress.").await.unwrap();
        expect(page.locator(".alert-error")).to_contain_text("Reference:").await.unwrap();
        expect(page.locator(".alert-error")).to_contain_text("0123456789abcdef0123456789abcdef").await.unwrap();
        capture(page,"nhs-dmd-failed-reference-desktop").await;
        for width in [390] {
            page.set_viewport_size(Viewport {width,height:844}).await.unwrap();
            capture(page,"nhs-dmd-failed-reference-mobile").await;
        }
        journey.admin.execute_unprepared("DELETE FROM nhs_dmd_imports WHERE id=92001").await.unwrap();
    })).await;
}
