use super::Application;
use med_tracker::models::{
    _entities::{household_memberships, people},
    access::{Actor, HouseholdScope},
    data_imports::{
        confirmation,
        consents::{self, ConsentBinding, Decision},
        contracts::{
            ConsentInput, ImportState, PersonDestination, PersonMapping, RecordKind, SourceSystem,
        },
        identity::RecipientConsent,
        inspection::{self, Upload},
        preview::{self, PreviewInput},
    },
    identity::credentials::{CredentialMethod, CredentialProvenance},
};
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel, Set};
use std::io::{Cursor, Write};
use zip::{ZipWriter, write::SimpleFileOptions};

fn upload(comment: &str) -> Upload {
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
    Upload {
        source: SourceSystem::MedTracker05,
        archive: zip.finish().unwrap().into_inner(),
        passphrase: "password".into(),
    }
}

fn profile_upload(comment: &str) -> Upload {
    let mut payload: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../fixtures/portable_rails/prepared-history-v2.json"
    ))
    .unwrap();
    let records = payload["records"].as_object_mut().unwrap();
    for (key, value) in records.iter_mut() {
        if key != "people" {
            *value = serde_json::json!([]);
        }
    }
    let person = &mut records.get_mut("people").unwrap()[0];
    person["person_type"] = serde_json::json!("adult");
    person["has_capacity"] = serde_json::json!(true);
    person["notification_preference_portable_id"] = serde_json::Value::Null;
    let mut manifest: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../fixtures/portable_rails/prepared-history-v2.manifest.json"
    ))
    .unwrap();
    for (key, count) in manifest["record_counts"].as_object_mut().unwrap() {
        *count = serde_json::json!(if key == "people" { 1 } else { 0 });
    }
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    zip.set_comment(comment);
    for (name, bytes) in [
        ("portable.json", serde_json::to_vec(&payload).unwrap()),
        ("manifest.json", serde_json::to_vec(&manifest).unwrap()),
    ] {
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(&bytes).unwrap();
    }
    Upload {
        source: SourceSystem::MedTracker05,
        archive: zip.finish().unwrap().into_inner(),
        passphrase: "password".into(),
    }
}

#[tokio::test]
async fn dependent_adult_can_reload_own_report_without_guardian_eligibility() {
    let app = Application::new().await;
    let scope = app.fixture.scenario.scope();
    let person = people::Entity::find_by_id(app.fixture.account.person_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let mut update = person.into_active_model();
    update.person_type = Set(2);
    update.has_capacity = Set(false);
    update.update(&app.fixture.admin).await.unwrap();
    let comment = uuid::Uuid::new_v4().to_string();
    let zone = inspection::confirmed_timezone("Europe/London", true).unwrap();
    let inspected = inspection::inspect(&app.context, &scope, profile_upload(&comment), zone)
        .await
        .unwrap();
    let source = inspected
        .inventory
        .records
        .iter()
        .find(|row| row.identity.kind == RecordKind::Person)
        .unwrap()
        .identity
        .clone();
    let subject =
        med_tracker::models::data_imports::contracts::PersonId(app.fixture.account.person_id);
    let result = preview::create(
        &app.context,
        &scope,
        PreviewInput {
            inspection_id: inspected.id,
            upload: profile_upload(&comment),
            timezone: zone,
            mappings: vec![PersonMapping {
                source,
                destination: PersonDestination::Existing(subject),
            }],
            declarations: vec![ConsentInput::ActingPerson {
                subject,
                accepted: true,
            }],
            recipient_consents: vec![],
            acknowledged_omissions: inspected
                .inventory
                .omissions
                .iter()
                .map(|item| item.acknowledgement_id.clone())
                .collect(),
        },
    )
    .await
    .unwrap();
    let token = app.token().await;
    let url = format!(
        "{}/api/v1/households/{}/data_imports/{}",
        app.origin, scope.household_id, result.report.id
    );
    assert_eq!(
        app.client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        200
    );
    app.close().await;
}

#[tokio::test]
async fn other_adult_profile_report_requires_consent_for_exact_archive_and_withdrawal_hides_it() {
    let app = Application::new().await;
    let recipient = app.fixture.foreign_household().await;
    let scope = app.fixture.scenario.scope();
    let original_profile = people::Entity::find_by_id(recipient.account.person_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let now = chrono::Utc::now().naive_utc();
    household_memberships::ActiveModel {
        household_id: Set(scope.household_id),
        person_id: Set(recipient.account.person_id),
        role: Set("member".into()),
        status: Set("active".into()),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let recipient_scope = HouseholdScope {
        actor: Actor {
            account_id: recipient.account.id,
        },
        household_id: scope.household_id,
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    let comment = uuid::Uuid::new_v4().to_string();
    let zone = inspection::confirmed_timezone("Europe/London", true).unwrap();
    let inspected = inspection::inspect(&app.context, &scope, profile_upload(&comment), zone)
        .await
        .unwrap();
    let source = inspected
        .inventory
        .records
        .iter()
        .find(|row| row.identity.kind == RecordKind::Person)
        .unwrap()
        .identity
        .clone();
    let target =
        med_tracker::models::data_imports::contracts::PersonId(recipient.account.person_id);
    let binding = ConsentBinding {
        source_sha256: inspected.archive_sha256.clone(),
        source_person: source.clone(),
        target_person: target,
    };
    let requester = consents::begin_for_target(&app.fixture.runtime, &scope, None, target)
        .await
        .unwrap();
    let consent_id = consents::request(&requester, binding.clone())
        .await
        .unwrap();
    requester.commit().await.unwrap();
    let tenant = consents::begin_for_actor(&app.fixture.runtime, &recipient_scope, None)
        .await
        .unwrap();
    consents::decide(&tenant, &consent_id, &binding, Decision::Approved)
        .await
        .unwrap();
    tenant.commit().await.unwrap();
    let result = preview::create(
        &app.context,
        &scope,
        PreviewInput {
            inspection_id: inspected.id,
            upload: profile_upload(&comment),
            timezone: zone,
            mappings: vec![PersonMapping {
                source: source.clone(),
                destination: PersonDestination::Existing(target),
            }],
            declarations: vec![],
            recipient_consents: vec![RecipientConsent {
                source_person: source.clone(),
                request_id: consent_id.clone(),
            }],
            acknowledged_omissions: inspected
                .inventory
                .omissions
                .iter()
                .map(|item| item.acknowledgement_id.clone())
                .collect(),
        },
    )
    .await
    .unwrap();
    let token = app.token().await;
    let url = format!(
        "{}/api/v1/households/{}/data_imports/{}",
        app.origin, scope.household_id, result.report.id
    );
    assert_eq!(
        app.client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        200
    );
    let current_profile = people::Entity::find_by_id(recipient.account.person_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current_profile.name, original_profile.name);
    assert_eq!(current_profile.email, original_profile.email);
    let other_comment = uuid::Uuid::new_v4().to_string();
    let other_inspection =
        inspection::inspect(&app.context, &scope, profile_upload(&other_comment), zone)
            .await
            .unwrap();
    assert_ne!(other_inspection.archive_sha256, binding.source_sha256);
    let other = preview::create(
        &app.context,
        &scope,
        PreviewInput {
            inspection_id: other_inspection.id,
            upload: profile_upload(&other_comment),
            timezone: zone,
            mappings: vec![PersonMapping {
                source: source.clone(),
                destination: PersonDestination::Existing(target),
            }],
            declarations: vec![],
            recipient_consents: vec![RecipientConsent {
                source_person: source.clone(),
                request_id: consent_id.clone(),
            }],
            acknowledged_omissions: other_inspection
                .inventory
                .omissions
                .iter()
                .map(|item| item.acknowledgement_id.clone())
                .collect(),
        },
    )
    .await
    .unwrap();
    let other_url = format!(
        "{}/api/v1/households/{}/data_imports/{}",
        app.origin, scope.household_id, other.report.id
    );
    assert_eq!(
        app.client
            .get(&other_url)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        404
    );
    let base = format!(
        "{}/api/v1/households/{}/data_imports",
        app.origin, scope.household_id
    );
    let mut cursor = None;
    let mut visible = Vec::new();
    for _ in 0..3 {
        let url = match &cursor {
            Some(id) => format!("{base}?limit=1&after_id={id}"),
            None => format!("{base}?limit=1"),
        };
        let response = app
            .client
            .get(url)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 200);
        let page: serde_json::Value = response.json().await.unwrap();
        for row in page["data"]["reports"].as_array().unwrap() {
            assert!(row.get("confirmation").is_none());
            visible.push(row["id"].as_str().unwrap().to_owned());
        }
        cursor = page["data"]["next_cursor"].as_str().map(str::to_owned);
        if cursor.is_none() {
            break;
        }
    }
    assert!(visible.contains(&result.report.id));
    assert!(!visible.contains(&other.report.id));
    let mut wrong_source = source.clone();
    wrong_source.id.push_str("-other");
    let wrong_binding = ConsentBinding {
        source_sha256: binding.source_sha256.clone(),
        source_person: wrong_source,
        target_person: target,
    };
    let requester = consents::begin_for_target(&app.fixture.runtime, &scope, None, target)
        .await
        .unwrap();
    let wrong_id = consents::request(&requester, wrong_binding.clone())
        .await
        .unwrap();
    requester.commit().await.unwrap();
    let tenant = consents::begin_for_actor(&app.fixture.runtime, &recipient_scope, None)
        .await
        .unwrap();
    consents::decide(&tenant, &wrong_id, &wrong_binding, Decision::Approved)
        .await
        .unwrap();
    tenant.commit().await.unwrap();
    let wrong = preview::create(
        &app.context,
        &scope,
        PreviewInput {
            inspection_id: inspection::inspect(
                &app.context,
                &scope,
                profile_upload(&comment),
                zone,
            )
            .await
            .unwrap()
            .id,
            upload: profile_upload(&comment),
            timezone: zone,
            mappings: vec![PersonMapping {
                source: source.clone(),
                destination: PersonDestination::Existing(target),
            }],
            declarations: vec![],
            recipient_consents: vec![RecipientConsent {
                source_person: source,
                request_id: wrong_id,
            }],
            acknowledged_omissions: vec![],
        },
    )
    .await
    .unwrap();
    let wrong_url = format!("{base}/{}", wrong.report.id);
    assert_eq!(
        app.client
            .get(wrong_url)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        404
    );
    let clinical_comment = uuid::Uuid::new_v4().to_string();
    let clinical = inspection::inspect(&app.context, &scope, upload(&clinical_comment), zone)
        .await
        .unwrap();
    let clinical_source = clinical
        .inventory
        .records
        .iter()
        .find(|row| row.identity.kind == RecordKind::Person)
        .unwrap()
        .identity
        .clone();
    let clinical_binding = ConsentBinding {
        source_sha256: clinical.archive_sha256.clone(),
        source_person: clinical_source.clone(),
        target_person: target,
    };
    let requester = consents::begin_for_target(&app.fixture.runtime, &scope, None, target)
        .await
        .unwrap();
    let clinical_id = consents::request(&requester, clinical_binding.clone())
        .await
        .unwrap();
    requester.commit().await.unwrap();
    let tenant = consents::begin_for_actor(&app.fixture.runtime, &recipient_scope, None)
        .await
        .unwrap();
    consents::decide(&tenant, &clinical_id, &clinical_binding, Decision::Approved)
        .await
        .unwrap();
    tenant.commit().await.unwrap();
    let clinical_report = preview::create(
        &app.context,
        &scope,
        PreviewInput {
            inspection_id: clinical.id,
            upload: upload(&clinical_comment),
            timezone: zone,
            mappings: vec![PersonMapping {
                source: clinical_source.clone(),
                destination: PersonDestination::Existing(target),
            }],
            declarations: vec![],
            recipient_consents: vec![RecipientConsent {
                source_person: clinical_source,
                request_id: clinical_id,
            }],
            acknowledged_omissions: clinical
                .inventory
                .omissions
                .iter()
                .map(|item| item.acknowledgement_id.clone())
                .collect(),
        },
    )
    .await
    .unwrap();
    let clinical_url = format!("{base}/{}", clinical_report.report.id);
    assert_eq!(
        app.client
            .get(clinical_url)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        404
    );
    let tenant = consents::begin_for_actor(&app.fixture.runtime, &recipient_scope, None)
        .await
        .unwrap();
    consents::decide(&tenant, &consent_id, &binding, Decision::Rejected)
        .await
        .unwrap();
    tenant.commit().await.unwrap();
    assert_eq!(
        app.client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        404
    );
    recipient.close().await;
    app.close().await;
}

#[tokio::test]
async fn completed_created_dependent_report_is_hidden_after_guardian_withdrawal() {
    let app = Application::new().await;
    let scope = app.fixture.scenario.scope();
    let comment = uuid::Uuid::new_v4().to_string();
    let zone = inspection::confirmed_timezone("Europe/London", true).unwrap();
    let inspected = inspection::inspect(&app.context, &scope, upload(&comment), zone)
        .await
        .unwrap();
    let source = inspected
        .inventory
        .records
        .iter()
        .find(|row| row.identity.kind == RecordKind::Person)
        .unwrap()
        .identity
        .clone();
    let ready = preview::create(
        &app.context,
        &scope,
        PreviewInput {
            inspection_id: inspected.id,
            upload: upload(&comment),
            timezone: zone,
            mappings: vec![PersonMapping {
                source: source.clone(),
                destination: PersonDestination::NewDependent,
            }],
            declarations: vec![ConsentInput::InitialGuardianDeclaration {
                source_person: source,
                accepted: true,
            }],
            recipient_consents: vec![],
            acknowledged_omissions: inspected
                .inventory
                .omissions
                .iter()
                .map(|item| item.acknowledgement_id.clone())
                .collect(),
        },
    )
    .await
    .unwrap();
    let provenance = CredentialProvenance {
        method: CredentialMethod::BrowserSession,
        reference: "synthetic-session".into(),
    };
    let completed = confirmation::confirm(
        &app.context,
        &scope,
        &ready.report.id,
        ready.confirmation.unwrap(),
        "password".into(),
        &provenance,
        &scope.request_id,
    )
    .await
    .unwrap();
    assert_eq!(completed.state, ImportState::Completed);
    let token = app.token().await;
    let report_url = format!(
        "{}/api/v1/households/{}/data_imports/{}",
        app.origin, scope.household_id, completed.id
    );
    assert_eq!(
        app.client
            .get(&report_url)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        200
    );
    let declarations_url = format!("{}/api/v1/auth/guardian-declarations", app.origin);
    let declarations = app
        .client
        .get(&declarations_url)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(declarations.status().as_u16(), 200);
    let declarations: serde_json::Value = declarations.json().await.unwrap();
    let destination = &completed.people[0].person.destination.as_ref().unwrap().id;
    let person_id = match destination {
        med_tracker::models::data_imports::contracts::DestinationId::Person(id) => id.0.to_string(),
        _ => panic!("created dependent must have a person destination"),
    };
    let declaration = declarations["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["subject_person_id"].as_str() == Some(person_id.as_str()))
        .unwrap();
    let id = declaration["id"].as_str().unwrap();
    let revision = declaration["revision"].as_str().unwrap();
    let withdrawn = app
        .client
        .patch(format!("{declarations_url}/{id}"))
        .bearer_auth(&token)
        .json(&serde_json::json!({"guardian_declaration":{"withdrawn":true,"revision":revision}}))
        .send()
        .await
        .unwrap();
    assert_eq!(withdrawn.status().as_u16(), 200);
    let denied = app
        .client
        .get(&report_url)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status().as_u16(), 404);
    app.close().await;
}

#[tokio::test]
async fn ready_report_reload_requires_unchanged_expectation_and_creator() {
    let app = Application::new().await;
    let scope = app.fixture.scenario.scope();
    let comment = uuid::Uuid::new_v4().to_string();
    let zone = inspection::confirmed_timezone("Europe/London", true).unwrap();
    let inspected = inspection::inspect(&app.context, &scope, upload(&comment), zone)
        .await
        .unwrap();
    let source = inspected
        .inventory
        .records
        .iter()
        .find(|row| row.identity.kind == RecordKind::Person)
        .unwrap()
        .identity
        .clone();
    let ready = preview::create(
        &app.context,
        &scope,
        PreviewInput {
            inspection_id: inspected.id,
            upload: upload(&comment),
            timezone: zone,
            mappings: vec![PersonMapping {
                source: source.clone(),
                destination: PersonDestination::NewDependent,
            }],
            declarations: vec![ConsentInput::InitialGuardianDeclaration {
                source_person: source,
                accepted: true,
            }],
            recipient_consents: vec![],
            acknowledged_omissions: inspected
                .inventory
                .omissions
                .iter()
                .map(|item| item.acknowledgement_id.clone())
                .collect(),
        },
    )
    .await
    .unwrap();
    assert_eq!(ready.report.state, ImportState::Ready);
    let token = app.token().await;
    let base = format!(
        "{}/api/v1/households/{}/data_imports",
        app.origin, scope.household_id
    );
    let detail = app
        .client
        .get(format!("{base}/{}", ready.report.id))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(detail.status().as_u16(), 200);
    let detail: serde_json::Value = detail.json().await.unwrap();
    assert_eq!(
        detail["data"]["confirmation"],
        serde_json::to_value(&ready.confirmation).unwrap()
    );
    for limit in [0, 101] {
        let response = app
            .client
            .get(format!("{base}?limit={limit}"))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 422);
    }
    let page = app
        .client
        .get(format!("{base}?limit=1"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(page.status().as_u16(), 200);
    let page: serde_json::Value = page.json().await.unwrap();
    assert!(page["data"]["reports"][0].get("confirmation").is_none());
    let member = app.fixture.scenario.membership.clone();
    let mut changed = member.clone().into_active_model();
    changed.updated_at = Set(member.updated_at + chrono::Duration::seconds(1));
    changed.update(&app.fixture.admin).await.unwrap();
    let detail = app
        .client
        .get(format!("{base}/{}", ready.report.id))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(detail.status().as_u16(), 200);
    let detail: serde_json::Value = detail.json().await.unwrap();
    assert!(detail["data"]["confirmation"].is_null());
    let foreign = app.fixture.foreign_household().await;
    let now = chrono::Utc::now().naive_utc();
    household_memberships::ActiveModel {
        household_id: Set(scope.household_id),
        person_id: Set(foreign.account.person_id),
        role: Set("administrator".into()),
        status: Set("active".into()),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let service = app
        .context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let other_key = service
        .api_keys()
        .unwrap()
        .create(
            &foreign.account.id.to_string(),
            better_auth::server_api::CreateKeyOptions {
                config_id: Some("medtracker-personal".into()),
                name: Some("Report creator isolation".into()),
                expires_in: Some(3600.0),
                metadata: Some(serde_json::json!({"households":[scope.household_id]})),
                permissions: Some(std::collections::HashMap::from([(
                    "medtracker".into(),
                    vec!["care:read".into()],
                )])),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .key;
    let other_creator = app
        .client
        .get(format!("{base}/{}", ready.report.id))
        .bearer_auth(&other_key)
        .send()
        .await
        .unwrap();
    assert_eq!(other_creator.status().as_u16(), 404);
    let denied = app
        .client
        .get(format!(
            "{}/api/v1/households/{}/data_imports/{}",
            app.origin, foreign.household.id, ready.report.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert!(matches!(denied.status().as_u16(), 403 | 404));
    foreign.close().await;
    app.close().await;
}
