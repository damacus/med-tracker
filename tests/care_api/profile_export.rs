use super::*;
use med_tracker::models::{
    access::{self, Actor, HouseholdScope},
    care::profile_export,
};
use std::io::{Cursor, Read};

#[tokio::test]
async fn profile_export_uses_manage_grants_and_creates_a_readable_zip_without_identity_secrets() {
    let app = Application::new().await;
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-export".into(),
    };
    app.fixture.admin.execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001; UPDATE person_access_grants SET access_level='view' WHERE id=78001").await.unwrap();
    let tenant = access::begin(&app.context.db, &scope).await.unwrap();
    let download = profile_export::build(&tenant, "health_data_json")
        .await
        .unwrap();
    tenant.commit().await.unwrap();
    let value: Value = serde_json::from_slice(&download.bytes).unwrap();
    assert!(
        value["records"]
            .as_object()
            .unwrap()
            .values()
            .all(|rows| rows.as_array().unwrap().is_empty())
    );
    app.fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET access_level='manage' WHERE id=78001")
        .await
        .unwrap();
    let tenant = access::begin(&app.context.db, &scope).await.unwrap();
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
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT count(*) AS count FROM security_audit_events WHERE event_type='portable_data.exported' AND actor_account_id=71001 AND metadata->>'export_mode'='backup_zip' AND metadata->>'encrypted'='false'")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    app.fixture
        .admin
        .execute_unprepared(
            "UPDATE person_access_grants SET expires_at=now()-interval '1 minute' WHERE id=78001",
        )
        .await
        .unwrap();
    let tenant = access::begin(&app.context.db, &scope).await.unwrap();
    let expired = profile_export::build(&tenant, "health_data_json")
        .await
        .unwrap();
    tenant.commit().await.unwrap();
    let expired: Value = serde_json::from_slice(&expired.bytes).unwrap();
    assert!(expired["records"]["people"].as_array().unwrap().is_empty());
    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='owner' WHERE id=74001")
        .await
        .unwrap();
    let tenant = access::begin(&app.context.db, &scope).await.unwrap();
    let household = profile_export::build(&tenant, "health_data_json")
        .await
        .unwrap();
    tenant.commit().await.unwrap();
    let household: Value = serde_json::from_slice(&household.bytes).unwrap();
    assert_eq!(household["scope"], "household");
    assert!(household["records"]["people"].as_array().unwrap().len() > 1);
    app.fixture.admin.execute_unprepared("CREATE FUNCTION reject_profile_export_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='portable_data.exported' THEN RAISE EXCEPTION 'synthetic export audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_profile_export_audit BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION reject_profile_export_audit()").await.unwrap();
    let tenant = access::begin(&app.context.db, &scope).await.unwrap();
    assert!(
        profile_export::build(&tenant, "health_data_json")
            .await
            .is_err()
    );
    tenant.rollback().await.unwrap();
    app.close().await;
}
