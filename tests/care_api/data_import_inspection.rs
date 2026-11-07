use super::Application;
use med_tracker::models::{
    access,
    data_imports::{
        contracts::{
            ConsentInput, ImportState, PersonDestination, PersonId, PersonMapping, RecordKind,
            SourceSystem,
        },
        identity,
        inspection::{self, InspectionError, Upload},
        persistence, preview,
    },
    errors::OperationError,
};
use sea_orm::{ActiveModelTrait, IntoActiveModel, Set, TransactionTrait};
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

#[tokio::test]
async fn preview_persists_ready_and_blocked_metadata_for_the_exact_inspection() {
    let app = Application::new().await;
    let scope = app.fixture.scenario.scope();
    let zone = inspection::confirmed_timezone("Europe/London", true).unwrap();
    let inspected = inspection::inspect(&app.context, &scope, upload("preview"), zone)
        .await
        .unwrap();
    let source = inspected
        .inventory
        .records
        .iter()
        .find(|record| record.identity.kind == RecordKind::Person)
        .unwrap()
        .identity
        .clone();
    let mapping = PersonMapping {
        source: source.clone(),
        destination: PersonDestination::NewDependent,
    };
    let declaration = ConsentInput::InitialGuardianDeclaration {
        source_person: source.clone(),
        accepted: true,
    };
    let acknowledgements: Vec<_> = inspected
        .inventory
        .omissions
        .iter()
        .map(|item| item.acknowledgement_id.clone())
        .collect();
    let make_input =
        |comment: &str, declarations: Vec<ConsentInput>, acknowledged_omissions: Vec<String>| {
            preview::PreviewInput {
                inspection_id: inspected.id.clone(),
                upload: upload(comment),
                timezone: zone,
                mappings: vec![mapping.clone()],
                declarations,
                recipient_consents: vec![],
                acknowledged_omissions,
            }
        };
    let blocked = preview::create(
        &app.context,
        &scope,
        make_input("preview", vec![], acknowledgements.clone()),
    )
    .await
    .unwrap();
    assert_eq!(blocked.report.state, ImportState::Blocked);
    assert!(blocked.confirmation.is_none());
    assert!(
        blocked
            .report
            .issues
            .iter()
            .any(|issue| issue.source.as_ref() == Some(&source))
    );
    assert_eq!(blocked.report.people.len(), 1);
    let missing_ack = preview::create(
        &app.context,
        &scope,
        make_input("preview", vec![declaration.clone()], vec![]),
    )
    .await
    .unwrap();
    assert_eq!(missing_ack.report.state, ImportState::Blocked);
    assert!(missing_ack.confirmation.is_none());
    assert!(missing_ack.report.totals.decisions_required > 0);
    let ready = preview::create(
        &app.context,
        &scope,
        make_input("preview", vec![declaration], acknowledgements.clone()),
    )
    .await
    .unwrap();
    assert_eq!(ready.report.state, ImportState::Ready);
    assert!(ready.confirmation.is_some());
    assert_eq!(ready.report.source_sha256, inspected.archive_sha256);
    assert_eq!(
        usize::try_from(ready.report.totals.omissions).unwrap(),
        inspected.inventory.omissions.len()
    );
    assert_eq!(ready.report.totals.created, 0);
    assert_eq!(ready.report.totals.verified_links, 0);
    assert_eq!(ready.report.people.len(), 1);
    assert_eq!(
        usize::try_from(ready.report.totals.expected_create + ready.report.totals.expected_match)
            .unwrap(),
        inspected.inventory.records.len()
    );
    assert_eq!(
        ready.report.people[0].relationships.len() + ready.report.relationships.len(),
        inspected.inventory.relationships.len()
    );
    assert!(
        ready.report.people[0]
            .records
            .iter()
            .all(|item| item.actual.is_none())
    );
    assert!(
        ready
            .report
            .records
            .iter()
            .all(|item| item.actual.is_none())
    );
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    let saved = persistence::find(&tenant, &ready.report.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved.snapshot.report, ready.report);
    assert!(saved.snapshot.context.is_some());
    let blocked_saved = persistence::find(&tenant, &blocked.report.id)
        .await
        .unwrap()
        .unwrap();
    assert!(blocked_saved.snapshot.confirmation.is_none());
    tenant.rollback().await.unwrap();
    assert!(
        preview::create(
            &app.context,
            &scope,
            make_input("changed", vec![], acknowledgements.clone())
        )
        .await
        .is_err()
    );
    let mut wrong_zone = make_input("preview", vec![], acknowledgements.clone());
    wrong_zone.timezone = inspection::confirmed_timezone("UTC", true).unwrap();
    assert!(
        preview::create(&app.context, &scope, wrong_zone)
            .await
            .is_err()
    );
    let mut forged = acknowledgements.clone();
    forged.push("not-an-omission".into());
    assert!(
        preview::create(&app.context, &scope, make_input("preview", vec![], forged))
            .await
            .is_err()
    );
    let mut duplicate = acknowledgements.clone();
    duplicate.push(acknowledgements[0].clone());
    assert!(
        preview::create(
            &app.context,
            &scope,
            make_input("preview", vec![], duplicate)
        )
        .await
        .is_err()
    );
    let mut conflict_input = make_input("preview", vec![], acknowledgements.clone());
    conflict_input.mappings[0].destination =
        PersonDestination::Existing(PersonId(app.fixture.minor.id));
    let conflict = preview::create(&app.context, &scope, conflict_input)
        .await
        .unwrap();
    assert_eq!(conflict.report.state, ImportState::Blocked);
    assert!(conflict.confirmation.is_none());
    assert!(
        conflict
            .report
            .issues
            .iter()
            .any(|issue| issue.source.as_ref() == Some(&source))
    );

    let mut person = app.fixture.minor.clone().into_active_model();
    person.person_type = Set(2);
    person.has_capacity = Set(false);
    person.date_of_birth = Set(Some("1990-01-15".parse().unwrap()));
    person.update(&app.fixture.admin).await.unwrap();
    let mut no_consent_input = make_input("preview", vec![], acknowledgements);
    no_consent_input.mappings[0].destination =
        PersonDestination::Existing(PersonId(app.fixture.minor.id));
    let no_consent = preview::create(&app.context, &scope, no_consent_input)
        .await
        .unwrap();
    assert_eq!(no_consent.report.state, ImportState::Blocked);
    assert!(no_consent.confirmation.is_none());
    assert!(
        no_consent
            .report
            .issues
            .iter()
            .any(|issue| issue.source.as_ref() == Some(&source)
                && issue.reason
                    == med_tracker::models::data_imports::contracts::ReasonCode::ConsentRequired)
    );
    app.close().await;
}

#[tokio::test]
async fn identity_checks_exact_people_and_requires_new_dependent_declaration() {
    let app = Application::new().await;
    let scope = app.fixture.scenario.scope();
    let zone = inspection::confirmed_timezone("Europe/London", true).unwrap();
    let inspected = inspection::inspect(&app.context, &scope, upload("identity"), zone)
        .await
        .unwrap();
    let source = inspected
        .inventory
        .records
        .iter()
        .find(|record| record.identity.kind == RecordKind::Person)
        .unwrap()
        .identity
        .clone();
    let new_dependent = PersonMapping {
        source: source.clone(),
        destination: PersonDestination::NewDependent,
    };
    let choices = identity::IdentityChoices {
        mappings: std::slice::from_ref(&new_dependent),
        declarations: &[],
        recipient_consents: &[],
    };
    let result = identity::assess(
        &app.context,
        &scope,
        upload("identity"),
        &inspected.id,
        zone,
        choices,
    )
    .await;
    assert!(matches!(
        result,
        Err(InspectionError::Operation(
            OperationError::Validation { .. }
        ))
    ));
    let duplicates = [new_dependent.clone(), new_dependent];
    let declaration = ConsentInput::InitialGuardianDeclaration {
        source_person: source.clone(),
        accepted: true,
    };
    let choices = identity::IdentityChoices {
        mappings: &duplicates,
        declarations: std::slice::from_ref(&declaration),
        recipient_consents: &[],
    };
    let result = identity::assess(
        &app.context,
        &scope,
        upload("identity"),
        &inspected.id,
        zone,
        choices,
    )
    .await;
    assert!(matches!(
        result,
        Err(InspectionError::Operation(OperationError::Forbidden))
    ));
    let mapped = PersonMapping {
        source: source.clone(),
        destination: PersonDestination::NewDependent,
    };
    let choices = identity::IdentityChoices {
        mappings: std::slice::from_ref(&mapped),
        declarations: std::slice::from_ref(&declaration),
        recipient_consents: &[],
    };
    let accepted = identity::assess(
        &app.context,
        &scope,
        upload("identity"),
        &inspected.id,
        zone,
        choices,
    )
    .await
    .unwrap();
    assert_eq!(accepted.people.len(), 1);
    assert!(accepted.people[0].declaration_satisfied);
    assert!(!accepted.people[0].clinical_authority);
    let mismatch = PersonMapping {
        source,
        destination: PersonDestination::Existing(PersonId(app.fixture.adult.id)),
    };
    let choices = identity::IdentityChoices {
        mappings: std::slice::from_ref(&mismatch),
        declarations: &[],
        recipient_consents: &[],
    };
    let result = identity::assess(
        &app.context,
        &scope,
        upload("identity"),
        &inspected.id,
        zone,
        choices,
    )
    .await;
    assert!(
        matches!(result, Err(InspectionError::Operation(OperationError::Conflict { ref code, .. })) if code == "import_identity_conflict")
    );
}

fn upload(comment: &str) -> Upload {
    Upload {
        source: SourceSystem::MedTracker05,
        archive: archive(comment),
        passphrase: "password".into(),
    }
}

#[tokio::test]
async fn confirmation_rejects_wrong_tokens_and_replays_completed_result() {
    use med_tracker::models::_entities::oauth_grants;
    use med_tracker::models::entities::{adoption_fact, medication, membership, schedule, version};
    use med_tracker::models::{
        data_imports::{confirmation, contracts::PreviewToken, credential::ImportPrincipal},
        identity::{
            credentials::{CredentialMethod, CredentialProvenance},
            resource,
        },
    };
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, sea_query::Expr};
    let app = Application::new().await;
    let scope = app.fixture.scenario.scope();
    let comment = uuid::Uuid::new_v4().to_string();
    let zone = inspection::confirmed_timezone("Europe/London", true).unwrap();
    let inspected = inspection::inspect(&app.context, &scope, upload(&comment), zone)
        .await
        .unwrap();
    let person = inspected
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
        preview::PreviewInput {
            inspection_id: inspected.id,
            upload: upload(&comment),
            timezone: zone,
            mappings: vec![PersonMapping {
                source: person.clone(),
                destination: PersonDestination::NewDependent,
            }],
            declarations: vec![ConsentInput::InitialGuardianDeclaration {
                source_person: person,
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
    let token = ready.confirmation.unwrap();
    let credential = app.token().await;
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        "authorization",
        format!("Bearer {credential}").parse().unwrap(),
    );
    let principal = resource::authenticate(&app.fixture.runtime, &headers, None)
        .await
        .unwrap();
    let provenance = principal.provenance().clone();
    let before_memberships = membership::Entity::find()
        .filter(membership::Column::HouseholdId.eq(scope.household_id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let before_medications = medication::Entity::find()
        .filter(medication::Column::HouseholdId.eq(scope.household_id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let before_schedules = schedule::Entity::find()
        .filter(schedule::Column::HouseholdId.eq(scope.household_id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let before_versions = version::Entity::find()
        .filter(version::Column::HouseholdId.eq(scope.household_id))
        .filter(version::Column::RequestId.eq(scope.request_id.clone()))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let before_facts = adoption_fact::Entity::find()
        .filter(adoption_fact::Column::HouseholdId.eq(scope.household_id))
        .filter(adoption_fact::Column::RequestId.eq(scope.request_id.clone()))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let forged = [
        CredentialProvenance {
            method: CredentialMethod::BrowserSession,
            reference: "forged-browser".into(),
        },
        CredentialProvenance {
            method: provenance.method,
            reference: "different-credential".into(),
        },
    ];
    for attribution in forged {
        assert!(matches!(
            confirmation::confirm_checked(
                &app.context,
                &scope,
                &ready.report.id,
                token.clone(),
                "password".into(),
                &attribution,
                &scope.request_id,
                ImportPrincipal::Api(&principal)
            )
            .await,
            Err(InspectionError::Operation(OperationError::Forbidden))
        ));
    }
    let check = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    assert_eq!(
        persistence::find(&check, &ready.report.id)
            .await
            .unwrap()
            .unwrap()
            .snapshot
            .report
            .state,
        ImportState::Ready
    );
    check.rollback().await.unwrap();
    assert_eq!(
        version::Entity::find()
            .filter(version::Column::HouseholdId.eq(scope.household_id))
            .filter(version::Column::RequestId.eq(scope.request_id.clone()))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        before_versions
    );
    assert_eq!(
        adoption_fact::Entity::find()
            .filter(adoption_fact::Column::HouseholdId.eq(scope.household_id))
            .filter(adoption_fact::Column::RequestId.eq(scope.request_id.clone()))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        before_facts
    );
    assert_eq!(
        medication::Entity::find()
            .filter(medication::Column::HouseholdId.eq(scope.household_id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        before_medications
    );
    assert_eq!(
        schedule::Entity::find()
            .filter(schedule::Column::HouseholdId.eq(scope.household_id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        before_schedules
    );
    assert_eq!(
        membership::Entity::find()
            .filter(membership::Column::HouseholdId.eq(scope.household_id))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        before_memberships
    );
    let grants = oauth_grants::Entity::find()
        .filter(oauth_grants::Column::AccountId.eq(scope.actor.account_id))
        .filter(oauth_grants::Column::RevokedAt.is_null())
        .all(&app.fixture.admin)
        .await
        .unwrap();
    assert!(!grants.is_empty());
    let ids: Vec<_> = grants.iter().map(|grant| grant.id).collect();
    oauth_grants::Entity::update_many()
        .col_expr(
            oauth_grants::Column::RevokedAt,
            Expr::value(Some(chrono::Utc::now().naive_utc())),
        )
        .filter(oauth_grants::Column::Id.is_in(ids.clone()))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    assert!(matches!(
        confirmation::confirm_checked(
            &app.context,
            &scope,
            &ready.report.id,
            token.clone(),
            "password".into(),
            &provenance,
            &scope.request_id,
            ImportPrincipal::Api(&principal)
        )
        .await,
        Err(InspectionError::Operation(
            OperationError::Unauthenticated | OperationError::Forbidden
        ))
    ));
    let check = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    assert_eq!(
        persistence::find(&check, &ready.report.id)
            .await
            .unwrap()
            .unwrap()
            .snapshot
            .report
            .state,
        ImportState::Ready
    );
    check.rollback().await.unwrap();
    oauth_grants::Entity::update_many()
        .col_expr(
            oauth_grants::Column::RevokedAt,
            Expr::value(Option::<chrono::NaiveDateTime>::None),
        )
        .filter(oauth_grants::Column::Id.is_in(ids.clone()))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    assert!(
        confirmation::confirm(
            &app.context,
            &scope,
            &ready.report.id,
            token.clone(),
            "wrong-password".into(),
            &provenance,
            &scope.request_id
        )
        .await
        .is_err()
    );
    assert!(
        confirmation::confirm(
            &app.context,
            &scope,
            &ready.report.id,
            PreviewToken("wrong".into()),
            "password".into(),
            &provenance,
            &scope.request_id
        )
        .await
        .is_err()
    );
    let completed = confirmation::confirm_checked(
        &app.context,
        &scope,
        &ready.report.id,
        token.clone(),
        "password".into(),
        &provenance,
        &scope.request_id,
        ImportPrincipal::Api(&principal),
    )
    .await
    .unwrap();
    assert_eq!(completed.state, ImportState::Completed);
    let versions = version::Entity::find()
        .filter(version::Column::HouseholdId.eq(scope.household_id))
        .filter(version::Column::RequestId.eq(scope.request_id.clone()))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    for item_type in ["GuardianDeclaration", "Schedule"] {
        let audit = versions
            .iter()
            .find(|row| row.item_type == item_type && row.event == "create")
            .unwrap();
        assert_eq!(audit.audit_context["authentication_method"], "oauth");
        assert_eq!(
            audit.audit_context["session_reference"],
            format!("oauth_grant:{}", provenance.reference)
        );
        assert_eq!(
            audit.audit_context["actor_account_id"],
            scope.actor.account_id
        );
    }
    let facts = adoption_fact::Entity::find()
        .filter(adoption_fact::Column::HouseholdId.eq(scope.household_id))
        .filter(adoption_fact::Column::RequestId.eq(scope.request_id.clone()))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    assert!(!facts.is_empty());
    assert!(facts.iter().all(
        |fact| fact.authentication_method.as_deref() == Some("oauth")
            && fact.credential_reference.as_deref() == Some(provenance.reference.as_str())
            && fact.captured_by_account_id == scope.actor.account_id
    ));
    assert_eq!(
        completed.totals.created,
        ready.report.totals.expected_create
    );
    assert_eq!(
        completed.totals.verified_links,
        ready.report.totals.expected_link
    );
    let replay = confirmation::confirm_checked(
        &app.context,
        &scope,
        &ready.report.id,
        token.clone(),
        "password".into(),
        &provenance,
        &scope.request_id,
        ImportPrincipal::Api(&principal),
    )
    .await
    .unwrap();
    assert_eq!(replay, completed);
    let declarations_url = format!("{}/api/v1/auth/guardian-declarations", app.origin);
    let declarations = app
        .client
        .get(&declarations_url)
        .bearer_auth(&credential)
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
    let withdrawn = app.client.patch(format!("{declarations_url}/{}", declaration["id"].as_str().unwrap()))
        .bearer_auth(&credential)
        .json(&serde_json::json!({"guardian_declaration":{"withdrawn":true,"revision":declaration["revision"]}}))
        .send().await.unwrap();
    assert_eq!(withdrawn.status().as_u16(), 200);
    assert!(matches!(
        confirmation::confirm_checked(
            &app.context,
            &scope,
            &ready.report.id,
            token.clone(),
            "password".into(),
            &provenance,
            &scope.request_id,
            ImportPrincipal::Api(&principal)
        )
        .await,
        Err(InspectionError::Operation(
            OperationError::NotFound | OperationError::Forbidden
        ))
    ));
    oauth_grants::Entity::update_many()
        .col_expr(
            oauth_grants::Column::RevokedAt,
            Expr::value(Some(chrono::Utc::now().naive_utc())),
        )
        .filter(oauth_grants::Column::Id.is_in(ids))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    assert!(matches!(
        confirmation::confirm_checked(
            &app.context,
            &scope,
            &ready.report.id,
            token.clone(),
            "password".into(),
            &provenance,
            &scope.request_id,
            ImportPrincipal::Api(&principal)
        )
        .await,
        Err(InspectionError::Operation(
            OperationError::Unauthenticated | OperationError::Forbidden
        ))
    ));
    assert!(
        confirmation::confirm(
            &app.context,
            &scope,
            &ready.report.id,
            PreviewToken("wrong".into()),
            "password".into(),
            &provenance,
            &scope.request_id
        )
        .await
        .is_err()
    );
    app.close().await;
}

#[tokio::test]
async fn preview_rechecks_unmapped_actor_eligibility_after_profile_lock_wait() {
    let app = Application::new().await;
    let scope = app.fixture.scenario.scope();
    let timezone = inspection::confirmed_timezone("Europe/London", true).unwrap();
    let inspected = inspection::inspect(&app.context, &scope, upload("actor-lock"), timezone)
        .await
        .unwrap();
    let source = inspected
        .inventory
        .records
        .iter()
        .find(|record| record.identity.kind == RecordKind::Person)
        .unwrap()
        .identity
        .clone();
    let input = preview::PreviewInput {
        inspection_id: inspected.id,
        upload: upload("actor-lock"),
        timezone,
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
    };
    let profile_update = app.fixture.admin.begin().await.unwrap();
    let blocker = super::data_import_consents::backend_pid(&profile_update).await;
    let mut actor = app.fixture.adult.clone().into_active_model();
    actor.has_capacity = Set(false);
    actor.update(&profile_update).await.unwrap();
    let ctx = app.context.clone();
    let mut task = tokio::spawn(async move { preview::create(&ctx, &scope, input).await });
    let blocked = super::data_import_consents::wait_for_any_blocked_by(&app, blocker).await;
    profile_update.commit().await.unwrap();
    let result = tokio::time::timeout(std::time::Duration::from_secs(10), &mut task)
        .await
        .unwrap()
        .unwrap();
    assert!(blocked, "preview must wait for the actor profile update");
    assert!(matches!(
        result,
        Err(InspectionError::Operation(OperationError::Forbidden))
    ));
    app.close().await;
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
