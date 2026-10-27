use super::*;

use med_tracker::models::{
    access::{self},
    care::profile_export,
};
use std::io::{Cursor, Read};

#[tokio::test]
async fn profile_export_preserves_self_and_creates_a_zip_without_identity_secrets() {
    let app = Application::new().await;
    let scope = app.fixture.scope();
    app.fixture.member().await;
    let mut grant = app.fixture.grant.clone().into_active_model();
    grant.access_level = Set("view".into());
    grant.update(&app.fixture.admin).await.unwrap();
    let tenant = access::begin(&app.context.db, &scope, None).await.unwrap();
    let download = profile_export::build(&tenant, "health_data_json")
        .await
        .unwrap();
    tenant.commit().await.unwrap();
    let value: Value = serde_json::from_slice(&download.bytes).unwrap();
    assert_eq!(value["records"]["people"].as_array().unwrap().len(), 1);
    assert_eq!(value["records"]["people"][0]["name"], "Synthetic adult");
    let mut grant = app.fixture.grant.clone().into_active_model();
    grant.access_level = Set("manage".into());
    grant.update(&app.fixture.admin).await.unwrap();
    let tenant = access::begin(&app.context.db, &scope, None).await.unwrap();
    let download = profile_export::build(&tenant, "backup_zip").await.unwrap();
    tenant.commit().await.unwrap();
    let mut zip = zip::ZipArchive::new(Cursor::new(download.bytes)).unwrap();
    assert_eq!(zip.len(), 1);
    let mut json = String::new();
    zip.by_name("medtracker-backup.json")
        .unwrap()
        .read_to_string(&mut json)
        .unwrap();
    let value: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["format"], "medtracker.backup.v1");
    assert_eq!(value["records"]["people"].as_array().unwrap().len(), 1);
    assert_eq!(value["records"]["medications"].as_array().unwrap().len(), 1);
    assert!(!json.contains("password_hash"));
    assert!(!json.contains("push_subscriptions"));
    let count = super::portable_imports::audit_fields(
        &app,
        "portable_data.exported",
        &[("export_mode", "backup_zip"), ("encrypted", "false")],
    )
    .await;
    assert_eq!(count, 1);
    let mut grant = app.fixture.grant.clone().into_active_model();
    grant.expires_at = Set(Some(
        chrono::Utc::now().naive_utc() - chrono::Duration::minutes(1),
    ));
    grant.update(&app.fixture.admin).await.unwrap();
    let tenant = access::begin(&app.context.db, &scope, None).await.unwrap();
    let expired = profile_export::build(&tenant, "health_data_json")
        .await
        .unwrap();
    tenant.commit().await.unwrap();
    let expired: Value = serde_json::from_slice(&expired.bytes).unwrap();
    assert_eq!(expired["records"]["people"], value["records"]["people"]);
    let mut member = app.fixture.membership.clone().into_active_model();
    member.role = Set("owner".into());
    member.update(&app.fixture.admin).await.unwrap();
    let tenant = access::begin(&app.context.db, &scope, None).await.unwrap();
    let household = profile_export::build(&tenant, "health_data_json")
        .await
        .unwrap();
    tenant.commit().await.unwrap();
    let household: Value = serde_json::from_slice(&household.bytes).unwrap();
    assert_eq!(household["scope"], "household");
    assert!(household["records"]["people"].as_array().unwrap().len() > 1);
    // SeaORM Schema cannot define PostgreSQL trigger functions; this exclusive fault injection is removed below.
    app.fixture.admin.execute_unprepared("CREATE FUNCTION reject_profile_export_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='portable_data.exported' THEN RAISE EXCEPTION 'synthetic export audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_profile_export_audit BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION reject_profile_export_audit()").await.unwrap();
    let tenant = access::begin(&app.context.db, &scope, None).await.unwrap();
    assert!(
        profile_export::build(&tenant, "health_data_json")
            .await
            .is_err()
    );
    tenant.rollback().await.unwrap();
    app.fixture.admin.execute_unprepared("DROP TRIGGER reject_profile_export_audit ON security_audit_events; DROP FUNCTION reject_profile_export_audit()").await.unwrap();
    app.close().await;
}
