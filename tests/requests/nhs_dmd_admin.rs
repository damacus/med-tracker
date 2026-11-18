use super::*;
use med_tracker::models::_entities::{nhs_dmd_imports, platform_admins};
use sea_orm::{ActiveModelTrait, Set};

#[tokio::test]
async fn nhs_dmd_upload_link_requires_system_administrator_and_connects_administration_pages() {
    let (fixture, boot) = boot_shared_fixture().await;
    let mut request = server(&boot);
    install_browser_session(&mut request, &boot, fixture.account.id).await;
    let overview = format!("/households/{}/admin", fixture.household.slug);
    let before = request.get(&overview).await;
    assert!(!before.text().contains("href=\"/administration/nhs-dmd\""));
    assert_eq!(
        request.get("/administration/nhs-dmd").await.status_code(),
        403
    );
    platform_admin(&fixture).await;
    let after = request.get(&overview).await;
    assert!(after.text().contains("href=\"/administration/nhs-dmd\""));
    let platform = request.get("/administration/users").await;
    assert_eq!(platform.status_code(), 200);
    assert!(platform.text().contains("href=\"/administration/nhs-dmd\""));
    let importer = request.get("/administration/nhs-dmd").await;
    assert_eq!(importer.status_code(), 200);
    assert!(importer.text().contains("href=\"/administration/users\""));
    assert!(importer.text().contains("aria-current=\"page\">NHS dm+d"));
    drop(request);
    close_shared(fixture, boot).await;
}

async fn platform_admin(fixture: &Scenario) {
    let now = chrono::Utc::now().naive_utc();
    platform_admins::ActiveModel {
        account_id: Set(fixture.account.id),
        status: Set("active".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
}

fn import_row(
    trace_id: Option<&str>,
    status: i32,
    error: Option<&str>,
) -> nhs_dmd_imports::ActiveModel {
    let now = chrono::Utc::now().naive_utc();
    nhs_dmd_imports::ActiveModel {
        uploaded_filename: Set("release.zip".into()),
        status: Set(status),
        error_message: Set(error.map(str::to_owned)),
        trace_id: Set(trace_id.map(str::to_owned)),
        created_at: Set(now),
        updated_at: Set(now),
        total_records: Set(0),
        processed_records: Set(0),
        imported_count: Set(0),
        created_count: Set(0),
        updated_count: Set(0),
        unchanged_count: Set(0),
        skipped_count: Set(0),
        skipped_expired_count: Set(0),
        skipped_invalid_count: Set(0),
        skipped_missing_name_count: Set(0),
        ..Default::default()
    }
}

#[tokio::test]
async fn nhs_dmd_admin_shows_trace_reference_only_for_failed_imports() {
    let (fixture, boot) = boot_shared_fixture().await;
    let mut request = server(&boot);
    platform_admin(&fixture).await;
    install_browser_session(&mut request, &boot, fixture.account.id).await;
    let trace = "0123456789abcdef0123456789abcdef";
    import_row(
        Some(trace),
        5,
        Some("The import stopped making progress. Upload the release again."),
    )
    .insert(&fixture.admin)
    .await
    .unwrap();
    import_row(None, 4, None)
        .insert(&fixture.admin)
        .await
        .unwrap();
    import_row(Some("fedcba9876543210fedcba9876543210"), 4, None)
        .insert(&fixture.admin)
        .await
        .unwrap();
    let response = request.get("/administration/nhs-dmd").await;
    let html = response.text();
    drop(request);
    close_shared(fixture, boot).await;

    assert!(html.contains("Reference:"), "missing reference: {html}");
    assert!(html.contains(trace), "missing trace id: {html}");
    assert_eq!(html.matches("Reference:").count(), 1, "{html}");
    assert!(!html.contains("fedcba9876543210fedcba9876543210"), "{html}");
}
