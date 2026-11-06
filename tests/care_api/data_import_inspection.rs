use super::Application;
use med_tracker::models::{
    access,
    data_imports::{
        contracts::SourceSystem,
        identity,
        inspection::{self, Upload},
    },
};
use std::io::{Cursor, Write};
use zip::{ZipWriter, write::SimpleFileOptions};

fn archive(comment: &str) -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    zip.set_comment(comment);
    for (name, data) in [
        (
            "portable.json",
            include_bytes!("../fixtures/portable_rails/prepared-history-v2.json").as_slice(),
        ),
        (
            "manifest.json",
            include_bytes!("../fixtures/portable_rails/prepared-history-v2.manifest.json")
                .as_slice(),
        ),
    ] {
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(data).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

fn upload(comment: &str) -> Upload {
    Upload {
        source: SourceSystem::MedTracker05,
        archive: archive(comment),
        passphrase: "password".into(),
    }
}

#[tokio::test]
async fn inspection_binds_exact_upload_actor_household_and_confirmed_timezone() {
    let app = Application::new().await;
    let scope = app.fixture.scenario.scope();
    assert!(inspection::confirmed_timezone("Europe/London", false).is_err());
    assert!(inspection::confirmed_timezone("not-a-timezone", true).is_err());
    let timezone = inspection::confirmed_timezone("Europe/London", true).unwrap();
    let inspected = inspection::inspect(&app.context, &scope, upload("first"), timezone)
        .await
        .unwrap();
    assert!(!inspected.inventory.records.is_empty());
    let same = inspection::prepare_upload(&app.context, upload("first"))
        .await
        .unwrap();
    let changed = inspection::prepare_upload(&app.context, upload("second"))
        .await
        .unwrap();
    assert_eq!(same.archive_sha256, inspected.archive_sha256);
    assert_ne!(changed.archive_sha256, inspected.archive_sha256);
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    inspection::verify(&tenant, &inspected.id, &same.archive_sha256, timezone)
        .await
        .unwrap();
    assert!(
        inspection::verify(&tenant, &inspected.id, &changed.archive_sha256, timezone)
            .await
            .is_err()
    );
    let other_zone = inspection::confirmed_timezone("UTC", true).unwrap();
    assert!(
        inspection::verify(&tenant, &inspected.id, &same.archive_sha256, other_zone)
            .await
            .is_err()
    );
    tenant.rollback().await.unwrap();
    assert!(
        identity::assess(
            &app.context,
            &scope,
            upload("first"),
            &inspected.id,
            timezone,
            identity::IdentityChoices {
                mappings: &[],
                declarations: &[],
                recipient_consents: &[]
            }
        )
        .await
        .is_err()
    );
    let foreign = app.fixture.foreign_household().await;
    let tenant = access::begin(&foreign.runtime, &foreign.scope(), None)
        .await
        .unwrap();
    assert!(
        inspection::verify(&tenant, &inspected.id, &same.archive_sha256, timezone)
            .await
            .is_err()
    );
    tenant.rollback().await.unwrap();
}
