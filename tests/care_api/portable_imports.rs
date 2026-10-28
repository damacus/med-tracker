use super::*;
use med_tracker::models::_entities as rows;
use med_tracker::models::care::portable_crypto;
use med_tracker::models::entities::{location, location_membership, person};
use sea_orm::sea_query::extension::postgres::PgExpr;
use sea_orm::sea_query::{Alias, Expr, ExprTrait, Query};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

const PASSPHRASE: &str = "synthetic portable secret";
const PASSPHRASE_HEADER: &str = "x-medtracker-portable-passphrase";

#[tokio::test]
async fn portable_v3_rejects_legacy_person_location_links_before_any_writes() {
    let app = Application::new().await;
    let fixture_imported_person_id = format!("imported-person-{}", app.fixture.client_uuid);
    let fixture_imported_location_id = format!("imported-location-{}", app.fixture.client_uuid);
    let token = app.token().await;
    let before = import_effects(&app).await;
    let links = location_membership::Entity::find()
        .filter(location_membership::Column::HouseholdId.eq(app.fixture.household.id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    for format in [
        "medtracker.portable.v1",
        "medtracker.portable.v2",
        "medtracker.portable.v3",
    ] {
        let mut payload = importable_payload(&app);
        payload["format"] = json!(format);
        payload["records"]["people"][0]["location_portable_ids"] =
            json!([fixture_imported_location_id.as_str()]);
        let bundle = portable_crypto::encrypt(&payload, PASSPHRASE).unwrap();
        for endpoint in ["portable_imports/dry_run", "portable_imports"] {
            let response = app
                .client
                .post(import_url(&app, endpoint))
                .bearer_auth(&token)
                .header(PASSPHRASE_HEADER, PASSPHRASE)
                .json(&fresh_dependent_request(
                    bundle.clone(),
                    fixture_imported_person_id.as_str(),
                ))
                .send()
                .await
                .unwrap();
            assert_eq!(response.status().as_u16(), 422, "{format} {endpoint}");
            let body: Value = response.json().await.unwrap();
            assert!(
                str::contains(&body.to_string(), "location_portable_ids"),
                "{body}"
            );
            assert!(
                str::contains(&body.to_string(), "medtracker.portable.v3"),
                "{body}"
            );
            assert_eq!(import_effects(&app).await, before);
            assert_eq!(
                location_membership::Entity::find()
                    .filter(location_membership::Column::HouseholdId.eq(app.fixture.household.id))
                    .count(&app.fixture.admin)
                    .await
                    .unwrap(),
                links
            );
        }
    }
    app.close().await;
}

#[tokio::test]
async fn portable_v3_import_restores_labels_without_person_location_memberships() {
    let app = Application::new().await;
    let fixture_imported_person_id = format!("imported-person-{}", app.fixture.client_uuid);
    let fixture_imported_location_id = format!("imported-location-{}", app.fixture.client_uuid);
    let token = app.token().await;
    let mut payload = importable_payload(&app);
    payload["format"] = json!("medtracker.portable.v3");
    payload["records"]["people"][0]
        .as_object_mut()
        .unwrap()
        .remove("location_portable_ids");
    let bundle = portable_crypto::encrypt(&payload, PASSPHRASE).unwrap();
    let before = import_effects(&app).await;
    let links = location_membership::Entity::find()
        .filter(location_membership::Column::HouseholdId.eq(app.fixture.household.id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let mut applied = None;
    for (attempt, (endpoint, expected)) in [
        ("portable_imports/dry_run", 200),
        ("portable_imports", 201),
        ("portable_imports", 201),
    ]
    .into_iter()
    .enumerate()
    {
        let request = if attempt == 2 {
            json!({"bundle":bundle.clone()})
        } else {
            fresh_dependent_request(bundle.clone(), fixture_imported_person_id.as_str())
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
        assert_eq!(status, expected, "{endpoint}: {body}");
        assert!(
            body["data"]["errors"].as_array().unwrap().is_empty(),
            "{body}"
        );
        assert_eq!(
            location_membership::Entity::find()
                .filter(location_membership::Column::HouseholdId.eq(app.fixture.household.id))
                .count(&app.fixture.admin)
                .await
                .unwrap(),
            links
        );
        if attempt == 1 {
            applied = Some(import_effects(&app).await);
        } else if attempt == 2 {
            assert_eq!(Some(import_effects(&app).await), applied);
        }
    }
    assert_eq!(
        location::Entity::find()
            .filter(location::Column::PortableId.eq(fixture_imported_location_id.as_str()))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        person::Entity::find()
            .filter(
                person::Column::Id.in_subquery(
                    Query::select()
                        .column(rows::household_memberships::Column::PersonId)
                        .from(rows::household_memberships::Entity)
                        .and_where(
                            Expr::col(rows::household_memberships::Column::HouseholdId)
                                .eq(app.fixture.household.id)
                        )
                        .to_owned()
                )
            )
            .filter(person::Column::PortableId.eq(fixture_imported_person_id.as_str()))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        1
    );
    let after = import_effects(&app).await;
    for field in ["takes", "stock", "notifications", "deliveries", "jobs"] {
        assert_eq!(after[field], before[field], "{field}");
    }
    app.close().await;
}

fn import_url(app: &Application, endpoint: &str) -> String {
    let fixture_household_id = app.fixture.household.id;
    format!(
        "{}/api/v1/households/{fixture_household_id}/{endpoint}",
        app.origin
    )
}

async fn export_bundle(app: &Application, token: &str) -> Value {
    let fixture_household_id = app.fixture.household.id;
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/portable_export?version=3",
            app.origin
        ))
        .bearer_auth(token)
        .header(PASSPHRASE_HEADER, PASSPHRASE)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    response.json::<Value>().await.unwrap()["data"].clone()
}

async fn version_count(app: &Application) -> i64 {
    i64::try_from(app.fixture.versions().await.len()).unwrap()
}

#[tokio::test]
async fn portable_imports_validate_the_request_shape_and_credentials() {
    let app = Application::new().await;
    let foreign = app.fixture.foreign_household().await;
    let foreign_id = foreign.household.id;
    foreign.close().await;
    let token = app.token().await;
    let mut statuses = Vec::new();
    let mut messages = Vec::new();
    for (endpoint, body, passphrase) in [
        ("portable_imports/dry_run", json!({}), Some(PASSPHRASE)),
        ("portable_imports", json!({}), Some(PASSPHRASE)),
        (
            "portable_imports/dry_run",
            json!({"bundle": {"format": "x"}, "unexpected": true}),
            Some(PASSPHRASE),
        ),
        (
            "portable_imports/dry_run",
            json!({"bundle": {"format": "x"}}),
            None,
        ),
        (
            "portable_imports/dry_run",
            json!({"bundle": "not an envelope"}),
            Some(PASSPHRASE),
        ),
        (
            "portable_imports/dry_run",
            json!("not an object"),
            Some(PASSPHRASE),
        ),
    ] {
        let mut request = app
            .client
            .post(import_url(&app, endpoint))
            .bearer_auth(&token)
            .json(&body);
        if let Some(passphrase) = passphrase {
            request = request.header(PASSPHRASE_HEADER, passphrase);
        }
        let response = request.send().await.unwrap();
        statuses.push(response.status().as_u16());
        messages.push(
            response.json::<Value>().await.unwrap()["error"]["message"]
                .as_str()
                .unwrap()
                .to_owned(),
        );
    }
    let anonymous = app
        .client
        .post(import_url(&app, "portable_imports/dry_run"))
        .header(PASSPHRASE_HEADER, PASSPHRASE)
        .json(&json!({"bundle": {}}))
        .send()
        .await
        .unwrap();
    let anonymous_status = anonymous.status().as_u16();
    let foreign = app
        .client
        .post(format!(
            "{}/api/v1/households/{foreign_id}/portable_imports/dry_run",
            app.origin
        ))
        .bearer_auth(&token)
        .header(PASSPHRASE_HEADER, PASSPHRASE)
        .json(&json!({"bundle": {}}))
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
    app.close().await;
    assert_eq!(statuses, vec![400, 400, 422, 422, 422, 400]);
    assert_eq!(
        messages,
        vec![
            "Portable bundle is required",
            "Portable bundle is required",
            "Unsupported import request fields",
            "Portable passphrase header is required",
            "Portable bundle could not be decrypted",
            "Portable bundle is required",
        ]
    );
    assert_eq!(anonymous_status, 401);
    assert_eq!(foreign_status, 403);
}

fn importable_payload(app: &Application) -> Value {
    let fixture_imported_person_id = format!("imported-person-{}", app.fixture.client_uuid);
    let fixture_imported_location_id = format!("imported-location-{}", app.fixture.client_uuid);
    json!({
        "format": "medtracker.portable.v1",
        "scope": "household",
        "exported_at": "2026-10-11T10:00:00Z",
        "source_instance_id": "synthetic:test",
        "records": {
            "locations": [{
                "portable_id": fixture_imported_location_id.as_str(),
                "name": "Imported Pharmacy",
                "description": "restored from a portable bundle"
            }],
            "people": [{
                "portable_id": fixture_imported_person_id.as_str(),
                "name": "Imported Dependent Adult",
                "email": format!("imported.dependent.{}@example.test", app.fixture.client_uuid),
                "date_of_birth": "1990-01-15",
                "person_type": "dependent_adult",
                "has_capacity": false,
                "location_portable_ids": []
            }]
        }
    })
}

fn fresh_dependent_request(bundle: Value, person_portable_id: &str) -> Value {
    json!({"bundle":bundle,"initial_guardian_declarations":[{
        "person_portable_id":person_portable_id,"accepted":true
    }]})
}

pub(super) async fn import_effects(app: &Application) -> Value {
    let household = app.fixture.household.id;
    macro_rules! count {
        ($entity:ident) => {
            rows::$entity::Entity::find()
                .filter(rows::$entity::Column::HouseholdId.eq(household))
                .count(&app.fixture.admin)
                .await
                .unwrap()
        };
    }
    let people = rows::people::Entity::find()
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let medication = rows::medications::Entity::find_by_id(app.fixture.medication.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    // Queue jobs are instance-wide. Cases using this no-side-effect snapshot require whole-run exclusivity.
    json!({"takes":count!(medication_takes),"stock":format!("{:.2}",medication.current_supply.unwrap()),"people":people,
        "memberships":count!(household_memberships),"declarations":generic_count(app,"guardian_declarations",Some("origin_household_id")).await,
        "carers":count!(carer_relationships),"grants":count!(person_access_grants),"locations":count!(locations),
        "versions":app.fixture.versions().await.len(),"changes":count!(api_change_events),"notifications":count!(notification_events),
        "deliveries":count!(notification_deliveries),"jobs":generic_count(app,"pg_loco_queue",None).await})
}

async fn table_count(app: &Application, table: &str) -> i64 {
    match table {
        "locations" => i64::try_from(
            rows::locations::Entity::find()
                .filter(rows::locations::Column::HouseholdId.eq(app.fixture.household.id))
                .count(&app.fixture.admin)
                .await
                .unwrap(),
        )
        .unwrap(),
        _ => panic!("unsupported fixture table"),
    }
}

#[tokio::test]
async fn portable_import_effect_snapshot_detects_unlinked_person_rows() {
    let app = Application::new().await;
    let before = import_effects(&app).await;
    let mut orphan = app.fixture.adult.clone().into_active_model();
    orphan.id = sea_orm::ActiveValue::NotSet;
    orphan.portable_id = Set(uuid::Uuid::new_v4().to_string());
    orphan.name = Set(format!(
        "Unlinked synthetic person {}",
        app.fixture.client_uuid
    ));
    orphan.insert(&app.fixture.admin).await.unwrap();
    let after = import_effects(&app).await;
    assert_eq!(after["memberships"], before["memberships"]);
    assert_eq!(
        after["people"].as_u64().unwrap(),
        before["people"].as_u64().unwrap() + 1
    );
    app.close().await;
}

#[tokio::test]
async fn portable_import_dry_run_reports_counts_without_writing() {
    let app = Application::new().await;
    let fixture_imported_person_id = format!("imported-person-{}", app.fixture.client_uuid);
    let token = app.token().await;
    let bundle = portable_crypto::encrypt(&importable_payload(&app), PASSPHRASE).unwrap();
    let versions_before = version_count(&app).await;
    let effects_before = import_effects(&app).await;
    let locations_before = table_count(&app, "locations").await;
    let response = app
        .client
        .post(import_url(&app, "portable_imports/dry_run"))
        .bearer_auth(&token)
        .header(PASSPHRASE_HEADER, PASSPHRASE)
        .json(&fresh_dependent_request(
            bundle,
            fixture_imported_person_id.as_str(),
        ))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    let versions_after = version_count(&app).await;
    let locations_after = table_count(&app, "locations").await;
    let exported = export_bundle(&app, &token).await;
    let exported_people = portable_crypto::decrypt(&exported, PASSPHRASE).unwrap();
    assert_eq!(
        exported_people["records"]["people"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    let denied_roundtrip = app
        .client
        .post(import_url(&app, "portable_imports/dry_run"))
        .bearer_auth(&token)
        .header(PASSPHRASE_HEADER, PASSPHRASE)
        .json(&json!({"bundle": exported.clone()}))
        .send()
        .await
        .unwrap();
    let denied_status = denied_roundtrip.status().as_u16();
    let denied_body: Value = denied_roundtrip.json().await.unwrap();
    assert_eq!(
        denied_status, 403,
        "roundtrip without other-person authority: {denied_body}"
    );
    assert_eq!(denied_body["error"]["code"], "forbidden");
    assert_eq!(import_effects(&app).await, effects_before);
    assert_eq!(
        audit_fields(
            &app,
            "api.request",
            &[
                ("controller", "api/v1/portable_imports"),
                ("action", "dry_run"),
                ("status", "403")
            ]
        )
        .await,
        1
    );
    let mut grants = Vec::new();
    for subject in [app.fixture.minor.id, app.fixture.dependent.id] {
        grants.push(
            rows::person_access_grants::ActiveModel {
                household_id: Set(app.fixture.household.id),
                household_membership_id: Set(app.fixture.membership.id),
                person_id: Set(subject),
                access_level: Set("manage".into()),
                relationship_type: Set("parent".into()),
                expires_at: Set(Some(
                    chrono::Utc::now().naive_utc() + chrono::Duration::hours(1),
                )),
                created_at: Set(chrono::Utc::now().naive_utc()),
                updated_at: Set(chrono::Utc::now().naive_utc()),
                ..Default::default()
            }
            .insert(&app.fixture.admin)
            .await
            .unwrap(),
        );
    }
    assert_eq!(grants.len(), 2);
    let authorized_effects_before = import_effects(&app).await;
    let legacy_roundtrip = app
        .client
        .post(import_url(&app, "portable_imports/dry_run"))
        .bearer_auth(&token)
        .header(PASSPHRASE_HEADER, PASSPHRASE)
        .json(&json!({"bundle": exported}))
        .send()
        .await
        .unwrap();
    assert_eq!(legacy_roundtrip.status().as_u16(), 403);
    assert_eq!(import_effects(&app).await, authorized_effects_before);
    use med_tracker::models::entities::membership;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, sea_query::Expr};
    membership::Entity::update_many()
        .col_expr(membership::Column::Role, Expr::value("member"))
        .filter(membership::Column::Id.eq(app.fixture.membership.id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let self_bundle = export_bundle(&app, &token).await;
    let own_records = portable_crypto::decrypt(&self_bundle, PASSPHRASE).unwrap();
    assert_eq!(
        own_records["records"]["people"].as_array().unwrap().len(),
        1
    );
    membership::Entity::update_many()
        .col_expr(membership::Column::Role, Expr::value("administrator"))
        .filter(membership::Column::Id.eq(app.fixture.membership.id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let roundtrip = app
        .client
        .post(import_url(&app, "portable_imports/dry_run"))
        .bearer_auth(&token)
        .header(PASSPHRASE_HEADER, PASSPHRASE)
        .json(&json!({"bundle":self_bundle}))
        .send()
        .await
        .unwrap();
    let roundtrip_status = roundtrip.status().as_u16();
    let roundtrip_body: Value = roundtrip.json().await.unwrap();
    assert_eq!(import_effects(&app).await, authorized_effects_before);
    assert_eq!(audit_fields(&app, "portable_data.imported", &[]).await, 0);
    app.close().await;
    assert_eq!(status, 200);
    let data = &body["data"];
    assert_eq!(data["applied"], false);
    assert_eq!(data["counts"]["locations"], 1);
    assert_eq!(data["counts"]["people"], 1);
    assert_eq!(data["conflicts"].as_array().unwrap().len(), 0);
    assert_eq!(data["errors"].as_array().unwrap().len(), 0);
    assert_eq!(versions_before, versions_after);
    assert_eq!(locations_before, locations_after);
    assert_eq!(roundtrip_status, 200, "roundtrip body: {roundtrip_body}");
    let roundtrip_data = &roundtrip_body["data"];
    assert_eq!(roundtrip_data["applied"], false);
    assert_eq!(
        roundtrip_data["errors"].as_array().unwrap().len(),
        0,
        "roundtrip errors: {roundtrip_data}"
    );
    assert_eq!(
        roundtrip_data["conflicts"].as_array().unwrap().len(),
        0,
        "roundtrip conflicts: {roundtrip_data}"
    );
}

#[tokio::test]
async fn portable_import_apply_writes_records_and_audits_the_import() {
    let app = Application::new().await;
    let fixture_imported_person_id = format!("imported-person-{}", app.fixture.client_uuid);
    let fixture_imported_location_id = format!("imported-location-{}", app.fixture.client_uuid);
    let token = app.token().await;
    let bundle = portable_crypto::encrypt(&importable_payload(&app), PASSPHRASE).unwrap();
    let imported = audit_fields(&app, "portable_data.imported", &[]).await;
    let versions_before = version_count(&app).await;
    let effects_before = import_effects(&app).await;
    let response = app
        .client
        .post(import_url(&app, "portable_imports"))
        .bearer_auth(&token)
        .header(PASSPHRASE_HEADER, PASSPHRASE)
        .json(&fresh_dependent_request(
            bundle,
            fixture_imported_person_id.as_str(),
        ))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    let audits = audit_fields(&app, "portable_data.imported", &[]).await;
    let requests = audit_fields(
        &app,
        "api.request",
        &[
            ("controller", "api/v1/portable_imports"),
            ("action", "create"),
            ("status", "201"),
        ],
    )
    .await;
    let locations = rows::locations::Entity::find()
        .filter(rows::locations::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(rows::locations::Column::PortableId.eq(fixture_imported_location_id.as_str()))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let people = rows::people::Entity::find()
        .filter(rows::people::Column::PortableId.eq(fixture_imported_person_id.as_str()))
        .filter(
            rows::people::Column::Id.in_subquery(
                Query::select()
                    .column(rows::household_memberships::Column::PersonId)
                    .from(rows::household_memberships::Entity)
                    .and_where(
                        Expr::col(rows::household_memberships::Column::HouseholdId)
                            .eq(app.fixture.household.id),
                    )
                    .to_owned(),
            ),
        )
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let imported_person = person::Entity::find()
        .filter(
            person::Column::Id.in_subquery(
                Query::select()
                    .column(rows::household_memberships::Column::PersonId)
                    .from(rows::household_memberships::Entity)
                    .and_where(
                        Expr::col(rows::household_memberships::Column::HouseholdId)
                            .eq(app.fixture.household.id),
                    )
                    .to_owned(),
            ),
        )
        .filter(person::Column::PortableId.eq(fixture_imported_person_id.as_str()))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let memberships = location_membership::Entity::find()
        .filter(location_membership::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(location_membership::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(location_membership::Column::PersonId.eq(imported_person.id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let grants = rows::person_access_grants::Entity::find()
        .filter(rows::person_access_grants::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(
            rows::person_access_grants::Column::HouseholdMembershipId.eq(app.fixture.membership.id),
        )
        .filter(rows::person_access_grants::Column::PersonId.eq(imported_person.id))
        .filter(rows::person_access_grants::Column::AccessLevel.eq("manage"))
        .filter(rows::person_access_grants::Column::RevokedAt.is_null())
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let query = Query::select()
        .columns([
            "id",
            "guardian_person_id",
            "declared_by_account_id",
            "origin_household_id",
            "statement_version",
            "revision",
            "withdrawn_at",
            "declared_at",
        ])
        .from("guardian_declarations")
        .and_where(Expr::col("subject_person_id").eq(imported_person.id))
        .and_where(Expr::col("origin_household_id").eq(app.fixture.household.id))
        .to_owned();
    let declarations = app
        .fixture
        .admin
        .query_all_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap();
    assert_eq!(declarations.len(), 1);
    let declaration = &declarations[0];
    let declaration_id: i64 = declaration.try_get("", "id").unwrap();
    let member = rows::household_memberships::Entity::find()
        .filter(rows::household_memberships::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(rows::household_memberships::Column::PersonId.eq(imported_person.id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let carer = rows::carer_relationships::Entity::find()
        .filter(rows::carer_relationships::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(rows::carer_relationships::Column::PatientId.eq(imported_person.id))
        .filter(rows::carer_relationships::Column::GuardianDeclarationId.eq(declaration_id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let version = rows::versions::Entity::find()
        .filter(rows::versions::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(rows::versions::Column::ItemType.eq("GuardianDeclaration"))
        .filter(rows::versions::Column::ItemId.eq(declaration_id))
        .filter(rows::versions::Column::Event.eq("create"))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let audit = rows::security_audit_events::Entity::find()
        .filter(rows::security_audit_events::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(rows::security_audit_events::Column::EventType.eq("guardian_declaration.created"))
        .filter(
            Expr::col(rows::security_audit_events::Column::Metadata)
                .cast_json_field("id")
                .eq(declaration_id.to_string()),
        )
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let current_authority = declaration
        .try_get::<i64>("", "guardian_person_id")
        .unwrap()
        == app.fixture.adult.id
        && declaration
            .try_get::<i64>("", "declared_by_account_id")
            .unwrap()
            == app.fixture.account.id
        && declaration
            .try_get::<i64>("", "origin_household_id")
            .unwrap()
            == app.fixture.household.id
        && declaration.try_get::<i16>("", "statement_version").unwrap() == 1
        && declaration.try_get::<i64>("", "revision").unwrap() == 1
        && declaration
            .try_get::<Option<chrono::DateTime<chrono::FixedOffset>>>("", "withdrawn_at")
            .unwrap()
            .is_none()
        && declaration
            .try_get::<chrono::DateTime<chrono::FixedOffset>>("", "declared_at")
            .unwrap()
            > "2026-10-11T10:00:00Z"
                .parse::<chrono::DateTime<chrono::FixedOffset>>()
                .unwrap()
        && member.status == "active"
        && member.revoked_at.is_none()
        && carer.active
        && carer.carer_id == app.fixture.adult.id
        && audit.actor_account_id == Some(app.fixture.account.id)
        && audit.actor_membership_id == Some(app.fixture.membership.id)
        && audit.request_id == version.request_id;
    let evidence = rows::versions::Entity::find()
        .filter(rows::versions::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(rows::versions::Column::RequestId.eq(version.request_id.clone().unwrap()))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let mut version_counts = std::collections::BTreeMap::<String, u64>::new();
    let attributed = evidence.iter().all(|row| {
        row.event == "create"
            && row.whodunnit.as_deref() == Some(app.fixture.account.id.to_string().as_str())
            && row.actor_membership_id == Some(app.fixture.membership.id)
            && row.audit_context["actor_account_id"].as_i64() == Some(app.fixture.account.id)
            && row.request_id.is_some()
            && row.audit_context["request_id"].as_str() == row.request_id.as_deref()
    });
    for row in evidence {
        *version_counts.entry(row.item_type).or_default() += 1;
    }
    let versions = version_count(&app).await - versions_before;
    let changes = rows::api_change_events::Entity::find()
        .filter(rows::api_change_events::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(rows::api_change_events::Column::RecordPortableId.is_in([
            fixture_imported_location_id.as_str(),
            fixture_imported_person_id.as_str(),
        ]))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let effects_after = import_effects(&app).await;
    for field in ["takes", "stock", "notifications", "deliveries", "jobs"] {
        assert_eq!(
            effects_after[field], effects_before[field],
            "Import changed {field}"
        );
    }
    app.close().await;
    assert_eq!(status, 201);
    assert_eq!(body["data"]["applied"], true);
    assert_eq!(body["data"]["counts"]["locations"], 1);
    assert_eq!(body["data"]["counts"]["people"], 1);
    assert_eq!(locations, 1);
    assert_eq!(people, 1);
    assert_eq!(memberships, 0);
    assert_eq!(grants, 0);
    assert_eq!(declarations.len(), 1);
    assert!(current_authority);
    assert_eq!(
        json!(version_counts),
        json!({
            "Location":1,"Person":1,"GuardianDeclaration":1,"HouseholdMembership":1,"CarerRelationship":1
        })
    );
    assert!(attributed);
    assert_eq!(versions, 5);
    assert_eq!(changes, 2);
    assert_eq!(audits - imported, 1);
    assert_eq!(requests, 1);
}

async fn portable_id(app: &Application, table: &str, id: i64) -> String {
    match table {
        "people" => {
            rows::people::Entity::find_by_id(id)
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .unwrap()
                .portable_id
        }
        "medications" => {
            rows::medications::Entity::find_by_id(id)
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .unwrap()
                .portable_id
        }
        _ => panic!("unsupported fixture table"),
    }
}

#[tokio::test]
async fn portable_import_applies_nullable_and_sparse_rows() {
    let app = Application::new().await;
    let fixture_imported_schedule_id = format!("imported-schedule-{}", app.fixture.client_uuid);
    let fixture_imported_prefs_id = format!("imported-prefs-{}", app.fixture.client_uuid);
    let token = app.token().await;
    let person_pid = portable_id(&app, "people", app.fixture.adult.id).await;
    let medication_pid = portable_id(&app, "medications", app.fixture.medication.id).await;
    let payload = json!({
        "format": "medtracker.portable.v1",
        "scope": "household",
        "records": {
            "schedules": [{
                "portable_id": fixture_imported_schedule_id.as_str(),
                "person_portable_id": person_pid,
                "medication_portable_id": medication_pid,
                "schedule_type": "daily",
                "schedule_config": {"times": ["08:00"]},
                "start_date": null,
                "end_date": null,
                "dose_amount": null,
                "dose_unit": null,
                "active": true
            }],
            "notification_preferences": [{
                "portable_id": fixture_imported_prefs_id.as_str(),
                "person_portable_id": person_pid
            }]
        }
    });
    let bundle = portable_crypto::encrypt(&payload, PASSPHRASE).unwrap();
    let dry_run = app
        .client
        .post(import_url(&app, "portable_imports/dry_run"))
        .bearer_auth(&token)
        .header(PASSPHRASE_HEADER, PASSPHRASE)
        .json(&json!({"bundle": bundle}))
        .send()
        .await
        .unwrap();
    let dry_status = dry_run.status().as_u16();
    let dry_body: Value = dry_run.json().await.unwrap();
    let response = app
        .client
        .post(import_url(&app, "portable_imports"))
        .bearer_auth(&token)
        .header(PASSPHRASE_HEADER, PASSPHRASE)
        .json(&json!({"bundle": bundle}))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    let prefs = rows::notification_preferences::Entity::find()
        .filter(rows::notification_preferences::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(
            rows::notification_preferences::Column::PortableId
                .eq(fixture_imported_prefs_id.as_str()),
        )
        .filter(rows::notification_preferences::Column::Enabled.eq(true))
        .filter(rows::notification_preferences::Column::DoseDueEnabled.eq(true))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    app.close().await;
    assert_eq!(dry_status, 200, "dry-run body: {dry_body}");
    assert_eq!(dry_body["data"]["errors"].as_array().unwrap().len(), 0);
    assert_eq!(status, 201, "apply body: {body}");
    assert_eq!(prefs, 1, "sparse preference row must apply column defaults");
}

#[tokio::test]
async fn portable_import_accepts_derived_occurrence_window_ends() {
    let app = Application::new().await;
    let token = app.token().await;
    let now = chrono::Utc::now().naive_utc();
    let schedule = rows::schedules::ActiveModel {
        household_id: Set(app.fixture.household.id),
        person_id: Set(app.fixture.adult.id),
        medication_id: Set(app.fixture.medication.id),
        schedule_type: Set(0),
        schedule_config: Set(json!({})),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let occurrence = rows::medication_dose_occurrences::ActiveModel {
        household_id: Set(app.fixture.household.id),
        schedule_id: Set(Some(schedule.id)),
        window_starts_on: Set("2026-10-06".parse().unwrap()),
        position: Set(1),
        outcome: Set("open".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let payload = json!({
        "format": "medtracker.portable.v2",
        "scope": "household",
        "records": {
            "dose_occurrences": [{
                "portable_id": occurrence.portable_id,
                "source_type": "schedule",
                "source_portable_id": schedule.portable_id,
                "window_starts_on": "2026-10-06",
                "window_ends_on": "2026-10-06",
                "position": 1,
                "scheduled_at": null,
                "outcome": "open",
                "reason": null,
                "note": null,
                "resolved_at": null,
                "medication_take_portable_id": null
            }]
        }
    });
    let bundle = portable_crypto::encrypt(&payload, PASSPHRASE).unwrap();
    let response = app
        .client
        .post(import_url(&app, "portable_imports/dry_run"))
        .bearer_auth(&token)
        .header(PASSPHRASE_HEADER, PASSPHRASE)
        .json(&json!({"bundle": bundle}))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    app.close().await;
    assert_eq!(status, 200, "dry-run body: {body}");
    assert_eq!(
        body["data"]["errors"].as_array().unwrap().len(),
        0,
        "derived window_ends_on must not conflict: {body}"
    );
}

#[tokio::test]
async fn portable_import_preflight_projects_pause_closures() {
    let app = Application::new().await;
    let token = app.token().await;
    let person_pid = portable_id(&app, "people", app.fixture.adult.id).await;
    let medication_pid = portable_id(&app, "medications", app.fixture.medication.id).await;
    let now = chrono::Utc::now().naive_utc();
    let schedule = rows::schedules::ActiveModel {
        household_id: Set(app.fixture.household.id),
        person_id: Set(app.fixture.adult.id),
        medication_id: Set(app.fixture.medication.id),
        schedule_type: Set(0),
        schedule_config: Set(json!({})),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let pause = rows::medication_pause_periods::ActiveModel {
        household_id: Set(app.fixture.household.id),
        schedule_id: Set(Some(schedule.id)),
        reason: Set("side_effects".into()),
        started_at: Set(Some("2026-10-01T09:00:00".parse().unwrap())),
        imported_context: Set(true),
        legacy_context: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let payload = json!({
        "format": "medtracker.portable.v2",
        "scope": "household",
        "records": {
            "schedules": [{
                "portable_id": schedule.portable_id,
                "person_portable_id": person_pid,
                "medication_portable_id": medication_pid,
                "schedule_type": "daily",
                "schedule_config": {"times": ["08:00"]},
                "active": false
            }],
            "medication_pause_periods": [{
                "portable_id": pause.portable_id,
                "source_type": "schedule",
                "source_portable_id": schedule.portable_id,
                "reason": "side_effects",
                "note": null,
                "started_at": "2026-10-01T09:00:00Z",
                "ended_at": "2026-10-05T09:00:00Z",
                "legacy_context": false
            }]
        }
    });
    let bundle = portable_crypto::encrypt(&payload, PASSPHRASE).unwrap();
    let response = app
        .client
        .post(import_url(&app, "portable_imports/dry_run"))
        .bearer_auth(&token)
        .header(PASSPHRASE_HEADER, PASSPHRASE)
        .json(&json!({"bundle": bundle}))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    app.close().await;
    assert_eq!(status, 200, "dry-run body: {body}");
    assert!(
        body["data"]["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|error| error.as_str().is_some_and(
                |message| message.contains("inactive source requires an open pause period")
            )),
        "closing a source's only open pause must surface the inactive-source error: {body}"
    );
}

#[tokio::test]
async fn portable_import_rejects_unsupported_and_inconsistent_payloads() {
    let app = Application::new().await;
    let token = app.token().await;
    let mut statuses = Vec::new();
    let mut messages = Vec::new();
    let effects_before = import_effects(&app).await;
    for (index, payload) in [
        json!({"format": "medtracker.portable.v4", "records": {}}),
        json!({"format": "medtracker.portable.v1"}),
        json!({"format": "medtracker.portable.v1", "records": {"widgets": []}, "unexpected": true}),
        json!({"format": "medtracker.portable.v1", "records": {"widgets": []}}),
        json!({"format": "medtracker.portable.v1", "records": {"people": [{"name": "No ID"}]}}),
        json!({"format": "medtracker.portable.v1", "records": {"people": [{"portable_id": "p-1", "bogus": 1}]}}),
        json!({"format": "medtracker.portable.v1", "records": {"people": [{"portable_id": "p-1", "id": 5, "name": "Dependent", "date_of_birth": "1990-01-01", "person_type": "dependent_adult", "has_capacity": false}]}}),
        json!({"format": "medtracker.portable.v1", "records": {"people": [{"portable_id": "p-adult", "name": "Capable adult must join", "date_of_birth": "1990-01-01", "person_type": "adult", "has_capacity": true}]}}),
    ].into_iter().enumerate() {
        let bundle = portable_crypto::encrypt(&payload, PASSPHRASE).unwrap();
        let request = match index {
            6 => fresh_dependent_request(bundle, "p-1"),
            7 => fresh_dependent_request(bundle, "p-adult"),
            _ => json!({"bundle":bundle}),
        };
        let response = app
            .client
            .post(import_url(&app, "portable_imports/dry_run"))
            .bearer_auth(&token)
            .header(PASSPHRASE_HEADER, PASSPHRASE)
            .json(&request)
            .send()
            .await
            .unwrap();
        statuses.push(response.status().as_u16());
        let body: Value = response.json().await.unwrap();
        messages.push(
            body["error"]["message"]
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| serde_json::to_string(&body["data"]["errors"]).unwrap()),
        );
    }
    assert_eq!(import_effects(&app).await, effects_before);
    app.close().await;
    assert_eq!(statuses, vec![422, 422, 422, 422, 422, 422, 200, 422]);
    assert_eq!(
        messages[..6],
        [
            "Unsupported portable data format",
            "Portable data records are required",
            "Unsupported portable payload fields",
            "Unsupported portable record type: widgets",
            "people[0].portable_id is required",
            "people[0] has unsupported fields",
        ]
    );
    assert!(str::contains(
        &messages[6],
        "includes Rails numeric IDs: id"
    ));
}

pub(super) async fn audit_fields(app: &Application, event: &str, fields: &[(&str, &str)]) -> i64 {
    let mut query = rows::security_audit_events::Entity::find()
        .filter(rows::security_audit_events::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(rows::security_audit_events::Column::EventType.eq(event));
    for (key, value) in fields {
        query = query.filter(
            Expr::col(rows::security_audit_events::Column::Metadata)
                .cast_json_field(*key)
                .eq(*value),
        );
    }
    i64::try_from(query.count(&app.fixture.admin).await.unwrap()).unwrap()
}

async fn generic_count(app: &Application, table: &str, household_column: Option<&str>) -> i64 {
    let mut query = Query::select();
    query
        .expr_as(Expr::col("id").count(), "count")
        .from(Alias::new(table));
    if let Some(column) = household_column {
        query.and_where(Expr::col(Alias::new(column)).eq(app.fixture.household.id));
    }
    app.fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap()
}
