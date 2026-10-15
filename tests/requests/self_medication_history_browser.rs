use super::self_medication_history::{
    clinical_effects, mobile_history_token, record_self_history_take, seed_self_history,
};
use super::*;

type BrowserResult = Result<(), Box<dyn std::error::Error>>;

async fn prepare_browser_history(
    fixture: &Fixture,
    boot: &BootResult,
    request: &mut TestServer,
) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    seed_self_history(fixture).await?;
    let token = mobile_history_token(request).await?;
    let first = record_self_history_take(
        request,
        &token,
        72001,
        81001,
        80001,
        "96800000-0000-4000-8000-000000000101",
        "2026-10-05T10:00:00.123456Z",
    )
    .await?;
    let second = record_self_history_take(
        request,
        &token,
        96821,
        96861,
        96851,
        "96800000-0000-4000-8000-000000000102",
        "2026-10-05T11:00:00Z",
    )
    .await?;
    record_self_history_take(
        request,
        &token,
        72001,
        96862,
        80001,
        "96800000-0000-4000-8000-000000000103",
        "2026-10-05T12:00:00Z",
    )
    .await?;
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=EXCLUDED.recovery_saved_at; UPDATE medications SET name='Later private inventory',friendly_name='Later private inventory' WHERE id IN(80001,96851); UPDATE person_medications SET retired_at=now(),active=false WHERE id IN(81001,96861); UPDATE household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE person_id=73001").await?;
    let service = boot
        .app_context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .ok_or("missing maintained identity service")?;
    let origin = url::Url::parse(&service.context().config.base_url)?
        .origin()
        .ascii_serialization();
    request.save_cookies();
    let sign_in = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", origin)
        .json(&json!({"email":"persistence@example.test","password":"password"}))
        .await;
    if sign_in.status_code() != 200 {
        return Err(format!(
            "maintained browser sign-in prerequisite returned {}",
            sign_in.status_code()
        )
        .into());
    }
    let profile = request.get("/profile").await;
    if profile.status_code() != 200 {
        return Err(format!(
            "zero-household account profile prerequisite returned {}",
            profile.status_code()
        )
        .into());
    }
    Ok(vec![first, second])
}

#[tokio::test]
async fn zero_household_browser_reads_its_original_medication_history() -> BrowserResult {
    let (fixture, boot) = boot_fixture().await;
    let mut request = server(&boot);
    let result = async {
        prepare_browser_history(&fixture, &boot, &mut request).await?;
        let before = clinical_effects(&fixture).await?;
        let profile = request.get("/profile").await;
        let page = request.get("/profile/medication-history").await;
        let cache = page
            .headers()
            .get("cache-control")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let after = clinical_effects(&fixture).await?;
        Ok::<_, Box<dyn std::error::Error>>((
            page.status_code().as_u16(),
            page.text(),
            cache,
            profile.text(),
            before,
            after,
        ))
    }
    .await;
    drop(request);
    close(fixture, boot).await;
    let (status, body, cache, profile, before, after) = result?;
    assert_eq!(before, after);
    assert_eq!(
        status, 200,
        "authenticated browser history route must render"
    );
    assert_eq!(cache.as_deref(), Some("no-store"));
    assert!(profile.contains("/profile/medication-history"));
    assert!(body.contains("<h1"));
    assert!(body.contains("Medication history"));
    assert!(body.contains("<table"));
    assert_eq!(body.matches("data-history-entry=").count(), 2);
    assert!(!body.contains("Synthetic dependent adult"));
    for label in [
        "Synthetic tablets",
        "Original second medicine",
        "Original self history subject",
    ] {
        assert!(body.contains(label));
    }
    for private in [
        "Later private inventory",
        "Private second source narrative marker",
        "Other person narrative marker",
        "actor_account_id",
    ] {
        assert!(!body.contains(private));
    }
    Ok(())
}

#[tokio::test]
async fn zero_household_browser_downloads_versioned_original_administrations() -> BrowserResult {
    let (fixture, boot) = boot_fixture().await;
    let mut request = server(&boot);
    let result = async {
        let originals = prepare_browser_history(&fixture, &boot, &mut request).await?;
        let before = clinical_effects(&fixture).await?;
        let download = request.get("/profile/medication-history/download").await;
        let cache = download
            .headers()
            .get("cache-control")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let disposition = download
            .headers()
            .get("content-disposition")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let after = clinical_effects(&fixture).await?;
        Ok::<_, Box<dyn std::error::Error>>((
            download.status_code().as_u16(),
            download.text(),
            cache,
            disposition,
            originals,
            before,
            after,
        ))
    }
    .await;
    drop(request);
    close(fixture, boot).await;
    let (status, body, cache, disposition, originals, before, after) = result?;
    assert_eq!(before, after);
    assert_eq!(
        status, 200,
        "authenticated administration download route must exist"
    );
    assert_eq!(cache.as_deref(), Some("no-store"));
    assert_eq!(
        disposition.as_deref(),
        Some("attachment; filename=\"medtracker-medication-history-v1.json\"")
    );
    let envelope: Value = serde_json::from_str(&body)?;
    assert_eq!(
        envelope
            .as_object()
            .ok_or("missing versioned history envelope")?
            .len(),
        4
    );
    assert_eq!(envelope["format"], "medtracker.personal_medication_history");
    assert_eq!(envelope["version"], 1);
    chrono::DateTime::parse_from_rfc3339(
        envelope["exported_at"]
            .as_str()
            .ok_or("missing export creation time")?,
    )?;
    let entries = envelope["data"]
        .as_array()
        .ok_or("missing original administration records")?;
    assert_eq!(entries.len(), 2);
    for (entry, original) in entries.iter().zip(originals) {
        contract::assert_value(
            contract::contract(),
            &contract::contract()["components"]["schemas"]["OwnMedicationHistoryEntry"],
            entry,
            "downloaded original administration",
        );
        assert_eq!(
            entry["id"],
            original["id"]
                .as_i64()
                .map(|id| Value::String(id.to_string()))
                .unwrap_or(original["id"].clone())
        );
    }
    for private in [
        "Later private inventory",
        "Private second source narrative marker",
        "Other person narrative marker",
        "actor_account_id",
        "credential",
    ] {
        assert!(!body.contains(private));
    }
    Ok(())
}
