use super::*;
use sea_orm::prelude::Decimal;
use std::str::FromStr;

#[test]
fn portable_rails_source_hashes_match_recorded_provenance() {
    use sha2::{Digest, Sha256};

    let provenance: Value =
        serde_json::from_str(include_str!("../fixtures/portable_rails/provenance.json")).unwrap();
    let sources: [(&str, &[u8]); 10] = [
        (
            "rails/app/services/households/hosted_export_authorization.rb",
            include_bytes!("../../rails/app/services/households/hosted_export_authorization.rb"),
        ),
        (
            "rails/app/services/households/hosted_export.rb",
            include_bytes!("../../rails/app/services/households/hosted_export.rb"),
        ),
        (
            "rails/app/services/api/record_etag.rb",
            include_bytes!("../../rails/app/services/api/record_etag.rb"),
        ),
        (
            "rails/app/services/portable_data/export_record_serializer.rb",
            include_bytes!("../../rails/app/services/portable_data/export_record_serializer.rb"),
        ),
        (
            "rails/app/services/portable_data/export_event_record_serializer.rb",
            include_bytes!(
                "../../rails/app/services/portable_data/export_event_record_serializer.rb"
            ),
        ),
        (
            "rails/app/services/portable_data/dose_occurrence_serializer.rb",
            include_bytes!("../../rails/app/services/portable_data/dose_occurrence_serializer.rb"),
        ),
        (
            "rails/app/services/portable_data/exporter.rb",
            include_bytes!("../../rails/app/services/portable_data/exporter.rb"),
        ),
        (
            "rails/app/services/portable_data/encryptor.rb",
            include_bytes!("../../rails/app/services/portable_data/encryptor.rb"),
        ),
        (
            "rails/config/application.rb",
            include_bytes!("../../rails/config/application.rb"),
        ),
        (
            "rails/Gemfile.lock",
            include_bytes!("../../rails/Gemfile.lock"),
        ),
    ];
    let recorded = provenance["source_files_sha256"].as_object().unwrap();
    assert_eq!(recorded.len(), sources.len());
    for (path, source) in sources {
        assert_eq!(
            recorded.get(path).and_then(Value::as_str),
            Some(hex::encode(Sha256::digest(source)).as_str()),
            "Retained source changed: {path}; review and regenerate the synthetic Rails fixtures"
        );
    }
}

#[tokio::test]
async fn prepared_rails_serializer_household_preserves_moved_stock_and_replays() {
    let prepared = med_tracker::models::clinical_transfer::prepare(
        include_bytes!("../fixtures/portable_rails/prepared-history-v2.json"),
        include_bytes!("../fixtures/portable_rails/prepared-history-v2.manifest.json"),
        PASSPHRASE,
    )
    .unwrap();
    let payload = portable_crypto::decrypt(&prepared.bundle, PASSPHRASE).unwrap();
    assert_eq!(payload["format"], "medtracker.portable.v3");
    restore_fixture(
        &serde_json::to_string(&payload).unwrap(),
        &serde_json::to_string(&prepared.bundle).unwrap(),
    )
    .await;
}

#[tokio::test]
async fn rails_serializer_household_payload_restores_and_replays_clinical_records() {
    restore_fixture(
        include_str!("../fixtures/portable_rails/household-v2.json"),
        include_str!("../fixtures/portable_rails/household-v2.encrypted.json"),
    )
    .await;
}

#[tokio::test]
async fn rails_serializer_migration_v1_restores_its_exported_collections() {
    restore_fixture(
        include_str!("../fixtures/portable_rails/migration-v1.json"),
        include_str!("../fixtures/portable_rails/migration-v1.encrypted.json"),
    )
    .await;
}

#[tokio::test]
async fn rails_serializer_migration_v2_restores_its_exported_collections() {
    restore_fixture(
        include_str!("../fixtures/portable_rails/migration-v2.json"),
        include_str!("../fixtures/portable_rails/migration-v2.encrypted.json"),
    )
    .await;
}

#[tokio::test]
async fn rails_serializer_obsolete_links_are_rejected_without_writes() {
    let app = Application::new().await;
    let bundle: Value = serde_json::from_str(include_str!(
        "../fixtures/portable_rails/linked-migration-v2.encrypted.json"
    ))
    .unwrap();
    let token = app.token().await;
    let before = import_effects(&app).await;
    for endpoint in ["portable_imports/dry_run", "portable_imports"] {
        let response = app
            .client
            .post(import_url(&app, endpoint))
            .bearer_auth(&token)
            .header(PASSPHRASE_HEADER, PASSPHRASE)
            .json(&fresh_dependent_request(
                bundle.clone(),
                "synthetic-rails-person-linked-migration-v2",
            ))
            .send()
            .await
            .unwrap();
        let status = response.status().as_u16();
        let body: Value = response.json().await.unwrap();
        assert_eq!(status, 422, "{body}");
        assert!(
            str::contains(&body.to_string(), "location_portable_ids"),
            "{body}"
        );
        assert_eq!(import_effects(&app).await, before);
    }
    app.close().await;
}

#[tokio::test]
async fn rails_serializer_v2_historical_location_gap_is_reported_without_writes() {
    let app = Application::new().await;
    let payload: Value = serde_json::from_str(include_str!(
        "../fixtures/portable_rails/historical-location-v2.json"
    ))
    .unwrap();
    assert_ne!(
        payload["records"]["medications"][0]["location_portable_id"],
        payload["records"]["medication_takes"][0]["taken_from_location_portable_id"]
    );
    let bundle: Value = serde_json::from_str(include_str!(
        "../fixtures/portable_rails/historical-location-v2.encrypted.json"
    ))
    .unwrap();
    assert_eq!(
        portable_crypto::decrypt(&bundle, PASSPHRASE).unwrap(),
        payload
    );
    let token = app.token().await;
    let before = import_effects(&app).await;
    for (endpoint, expected) in [("portable_imports/dry_run", 200), ("portable_imports", 422)] {
        let response = app
            .client
            .post(import_url(&app, endpoint))
            .bearer_auth(&token)
            .header(PASSPHRASE_HEADER, PASSPHRASE)
            .json(&fresh_dependent_request(
                bundle.clone(),
                payload["records"]["people"][0]["portable_id"]
                    .as_str()
                    .unwrap(),
            ))
            .send()
            .await
            .unwrap();
        let status = response.status().as_u16();
        let body: Value = response.json().await.unwrap();
        assert_eq!(status, expected, "{body}");
        assert_eq!(
            body["data"]["errors"],
            json!([
                "medication_takes[0].taken_from_location_portable_id does not match medication location"
            ])
        );
        assert_eq!(body["data"]["applied"], false);
        assert_eq!(import_effects(&app).await, before);
    }
    app.close().await;
}

async fn restore_fixture(payload: &str, envelope: &str) {
    let app = Application::new().await;
    let payload: Value = serde_json::from_str(payload).unwrap();
    let bundle: Value = serde_json::from_str(envelope).unwrap();
    assert_eq!(
        portable_crypto::decrypt(&bundle, PASSPHRASE).unwrap(),
        payload
    );
    let token = app.token().await;
    let before = import_effects(&app).await;
    let links_before = location_membership::Entity::find()
        .filter(location_membership::Column::HouseholdId.eq(app.fixture.household.id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let denied = app
        .client
        .post(import_url(&app, "portable_imports"))
        .bearer_auth(&token)
        .header(PASSPHRASE_HEADER, PASSPHRASE)
        .json(&json!({"bundle": bundle}))
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status().as_u16(), 422);
    assert_eq!(import_effects(&app).await, before);
    let mut applied = None;
    for (attempt, endpoint) in [
        "portable_imports/dry_run",
        "portable_imports",
        "portable_imports",
    ]
    .into_iter()
    .enumerate()
    {
        let request = if attempt == 2 {
            json!({"bundle": bundle})
        } else {
            fresh_dependent_request(
                bundle.clone(),
                payload["records"]["people"][0]["portable_id"]
                    .as_str()
                    .unwrap(),
            )
        };
        let response = app
            .client
            .post(import_url(&app, endpoint))
            .bearer_auth(&token)
            .header(PASSPHRASE_HEADER, PASSPHRASE)
            .json(&request)
            .send()
            .await
            .unwrap();
        let status = response.status().as_u16();
        let body: Value = response.json().await.unwrap();
        assert_eq!(status, if attempt == 0 { 200 } else { 201 }, "{body}");
        assert_eq!(body["data"]["errors"], json!([]), "{body}");
        for (kind, records) in payload["records"].as_object().unwrap() {
            assert_eq!(
                body["data"]["counts"][kind],
                records.as_array().unwrap().len(),
                "{kind}"
            );
        }
        let effects = import_effects(&app).await;
        if attempt == 0 {
            assert_eq!(effects, before);
        } else if attempt == 1 {
            applied = Some(effects);
        } else {
            assert_eq!(Some(effects), applied);
        }
    }
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{}/sync/snapshot",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let exported: Value = response.json().await.unwrap();
    for (kind, records) in payload["records"].as_object().unwrap() {
        for original in records.as_array().unwrap() {
            if kind == "locations" {
                let saved = location::Entity::find()
                    .filter(location::Column::HouseholdId.eq(app.fixture.household.id))
                    .filter(
                        location::Column::PortableId.eq(original["portable_id"].as_str().unwrap()),
                    )
                    .one(&app.fixture.admin)
                    .await
                    .unwrap()
                    .unwrap();
                assert_record(
                    kind,
                    original,
                    &json!({"portable_id":saved.portable_id,"name":saved.name,"description":saved.description}),
                );
                continue;
            }
            let restored = exported["data"]["records"][kind]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["portable_id"] == original["portable_id"])
                .unwrap_or_else(|| panic!("Missing {kind} {}", original["portable_id"]));
            assert_record(kind, original, restored);
        }
    }
    let after = import_effects(&app).await;
    for field in ["stock", "notifications", "deliveries", "jobs"] {
        assert_eq!(after[field], before[field], "{field}");
    }
    assert_eq!(
        location_membership::Entity::find()
            .filter(location_membership::Column::HouseholdId.eq(app.fixture.household.id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        links_before
    );
    app.close().await;
}

fn assert_record(kind: &str, original: &Value, restored: &Value) {
    for (field, value) in original.as_object().unwrap() {
        if ["etag", "updated_at", "location_portable_ids"].contains(&field.as_str()) {
            continue;
        }
        if field == "imported_context" {
            assert_eq!(restored[field], true);
        } else if [
            "dose_amount",
            "amount",
            "current_supply",
            "reorder_threshold",
            "default_min_hours_between_doses",
        ]
        .contains(&field.as_str())
            && !value.is_null()
        {
            let expected = Decimal::from_str(
                &value
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| value.to_string()),
            )
            .unwrap();
            let actual = Decimal::from_str(
                &restored[field]
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| restored[field].to_string()),
            )
            .unwrap();
            assert_eq!(actual, expected, "{kind}.{field}");
        } else {
            assert_eq!(&restored[field], value, "{kind}.{field}");
        }
    }
}
