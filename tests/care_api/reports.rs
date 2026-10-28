use super::*;
use sea_orm::sea_query::{Expr as QueryExpr, ExprTrait as _, Func as QueryFunc, Query};

#[tokio::test]
async fn reports_conformance_real_pdf_downloads_have_protected_headers_and_embedded_font() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let token = app.token().await;
    let mut results = Vec::new();
    for path in [
        format!(
            "/api/v1/households/{fixture_household_id}/reports/health_history.pdf?person_id={fixture_person_id}&start_date=2026-01-01&end_date=2026-01-31"
        ),
        format!(
            "/api/v1/households/{fixture_household_id}/reports/medication_reviews.pdf?person_id={fixture_person_id}"
        ),
    ] {
        let response = app
            .client
            .get(format!("{}{path}", app.origin))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        let status = response.status().as_u16();
        let headers = response.headers().clone();
        let bytes = response.bytes().await.unwrap();
        results.push((status, headers, bytes));
    }
    app.close().await;
    for (status, headers, bytes) in results {
        assert_eq!(status, 200);
        assert_eq!(headers["content-type"], "application/pdf");
        assert_eq!(headers["cache-control"], "no-store");
        assert!(
            headers["content-disposition"]
                .to_str()
                .unwrap()
                .starts_with("attachment;")
        );
        assert!(bytes.starts_with(b"%PDF-1.7"));
        assert!(
            bytes
                .windows(b"/FontFile2".len())
                .any(|window| window == b"/FontFile2")
        );
    }
}

#[tokio::test]
async fn reports_json_requires_current_person_access_and_valid_dates() {
    use med_tracker::models::entities::membership;
    use sea_orm::{
        ColumnTrait, EntityTrait, QueryFilter,
        sea_query::{Expr, ExprTrait},
    };
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_grant_id = app.fixture.grant.id;
    let foreign_household = app.fixture.seed_foreign_household(0).await;
    let token = app.token().await;
    let base = format!(
        "{}/api/v1/households/{fixture_household_id}/reports/health_history",
        app.origin
    );
    let visible = app
        .client
        .get(format!(
            "{base}?person_id={fixture_person_id}&start_date=2026-01-01&end_date=2026-01-31"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(visible.status().as_u16(), 200);
    assert_eq!(visible.headers()["cache-control"], "no-store");
    let body: Value = visible.json().await.unwrap();
    assert_eq!(body["data"]["person"]["id"], fixture_person_id.to_string());
    let take_stock_before = app.fixture.effect().await;
    let unmanaged = app
        .client
        .get(format!(
            "{base}?person_id={fixture_minor_id}&start_date=2026-01-01&end_date=2026-01-31"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let unmanaged_status = unmanaged.status().as_u16();
    let unmanaged_body: Value = unmanaged.json().await.unwrap();
    assert_eq!(unmanaged_status, 404, "{unmanaged_body}");
    assert_eq!(unmanaged_body["error"]["code"], "not_found");
    assert_eq!(unmanaged_body["error"]["message"], "Record not found");
    assert_eq!(app.fixture.effect().await, take_stock_before);
    let _person_access_grants_78002 =
        med_tracker::models::_entities::person_access_grants::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            household_membership_id: sea_orm::Set(fixture_membership_id),
            person_id: sea_orm::Set(fixture_minor_id),
            access_level: sea_orm::Set("manage".to_owned()),
            relationship_type: sea_orm::Set("parent".to_owned()),
            expires_at: sea_orm::Set(Some(
                (chrono::Utc::now() + chrono::Duration::hours(1)).naive_utc(),
            )),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(_person_access_grants_78002.person_id, fixture_minor_id);
    let legacy = app
        .client
        .get(format!(
            "{base}?person_id={fixture_minor_id}&start_date=2026-01-01&end_date=2026-01-31"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(legacy.status().as_u16(), 404);
    let created = app.client.post(format!("{}/api/v1/households/{fixture_household_id}/people", app.origin))
        .bearer_auth(&token).json(&json!({"person":{"name":"Synthetic report guardian child","date_of_birth":"2020-01-01","person_type":"minor","has_capacity":false,"initial_guardian_declaration":true}}))
        .send().await.unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let created: Value = created.json().await.unwrap();
    let subject = created["data"]["id"].as_i64().unwrap();
    let managed = app
        .client
        .get(format!(
            "{base}?person_id={subject}&start_date=2026-01-01&end_date=2026-01-31"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(managed.status().as_u16(), 200);
    let managed_body: Value = managed.json().await.unwrap();
    assert_eq!(managed_body["data"]["person"]["id"], subject.to_string());
    assert_eq!(app.fixture.effect().await, take_stock_before);
    membership::Entity::update_many()
        .col_expr(membership::Column::Status, Expr::value("revoked"))
        .col_expr(
            membership::Column::RevokedAt,
            Expr::value(chrono::Utc::now().naive_utc()),
        )
        .col_expr(
            membership::Column::PermissionsVersion,
            Expr::col(membership::Column::PermissionsVersion).add(1),
        )
        .filter(membership::Column::HouseholdId.eq(fixture_household_id))
        .filter(membership::Column::PersonId.eq(subject))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let departed = app
        .client
        .get(format!(
            "{base}?person_id={subject}&start_date=2026-01-01&end_date=2026-01-31"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(departed.status().as_u16(), 404);
    let foreign_person_id = med_tracker::models::_entities::accounts::Entity::find_by_id(
        foreign_household.created_by_account_id.unwrap(),
    )
    .one(&app.fixture.admin)
    .await
    .unwrap()
    .unwrap()
    .person_id;
    let foreign_reply=app.client.get(format!("{}/api/v1/households/{fixture_72002}/reports/health_history?person_id={foreign_person_id}&start_date=2026-01-01&end_date=2026-01-31",app.origin, fixture_72002=foreign_household.id)).bearer_auth(&token).send().await.unwrap();
    assert_eq!(foreign_reply.status().as_u16(), 403);
    let foreign_subject = app
        .client
        .get(format!(
            "{base}?person_id={foreign_person_id}&start_date=2026-01-01&end_date=2026-01-31"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(foreign_subject.status().as_u16(), 404);
    let invalid = app
        .client
        .get(format!(
            "{base}?person_id={fixture_person_id}&start_date=2026-01-31&end_date=2026-01-01"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(invalid.status().as_u16(), 422);
    med_tracker::models::_entities::person_access_grants::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::person_access_grants::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(Some((chrono::Utc::now()).naive_utc())),
        )
        .filter(
            med_tracker::models::_entities::person_access_grants::Column::Id.eq(fixture_grant_id),
        )
        .filter(
            med_tracker::models::_entities::person_access_grants::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let revoked = app
        .client
        .get(format!(
            "{base}?person_id={fixture_person_id}&start_date=2026-01-01&end_date=2026-01-31"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(revoked.status().as_u16(), 200);
    med_tracker::models::_entities::household_memberships::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::household_memberships::Column::Status,
            sea_orm::sea_query::Expr::value("revoked".to_owned()),
        )
        .col_expr(
            med_tracker::models::_entities::household_memberships::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(Some((chrono::Utc::now()).naive_utc())),
        )
        .col_expr(
            med_tracker::models::_entities::household_memberships::Column::PermissionsVersion,
            sea_orm::sea_query::Expr::col(
                med_tracker::models::_entities::household_memberships::Column::PermissionsVersion,
            )
            .add(1),
        )
        .filter(
            med_tracker::models::_entities::household_memberships::Column::Id
                .eq(fixture_membership_id),
        )
        .filter(
            med_tracker::models::_entities::household_memberships::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let actor_departed = app
        .client
        .get(format!(
            "{base}?person_id={fixture_person_id}&start_date=2026-01-01&end_date=2026-01-31"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(actor_departed.status().as_u16(), 403);
    app.close().await;
}

#[tokio::test]
async fn medication_review_summary_counts_only_people_with_visible_prompts() {
    use med_tracker::models::{
        access::{Actor, HouseholdScope, PersonAccess, begin},
        care::reports,
    };
    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_dependent_id = app.fixture.dependent.id;
    let scope = HouseholdScope {
        actor: Actor {
            account_id: fixture_account_id,
        },
        household_id: fixture_household_id,
        request_id: "synthetic-review-count".to_owned(),
    };
    let tenant = begin(&app.fixture.runtime, &scope, None).await.unwrap();
    let people = reports::accessible_people(&tenant, PersonAccess::View)
        .await
        .unwrap();
    assert_eq!(people.len(), 3);
    let mut participant_ids = people.iter().map(|person| person.id).collect::<Vec<_>>();
    participant_ids.sort_unstable();
    assert_eq!(
        participant_ids,
        vec![fixture_person_id, fixture_minor_id, fixture_dependent_id]
    );
    let report = reports::household_medication_reviews(
        &tenant,
        &people,
        None,
        chrono::Utc::now(),
        chrono::Utc::now().date_naive(),
    )
    .await
    .map(|report| serde_json::to_value(report).unwrap())
    .unwrap();
    assert_eq!(report["prompts"].as_array().unwrap().len(), 0);
    assert_eq!(report["people_count"], 0);
    tenant.rollback().await.unwrap();
    med_tracker::models::_entities::household_memberships::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::household_memberships::Column::Status,
            sea_orm::sea_query::Expr::value("revoked".to_owned()),
        )
        .col_expr(
            med_tracker::models::_entities::household_memberships::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(Some((chrono::Utc::now()).naive_utc())),
        )
        .col_expr(
            med_tracker::models::_entities::household_memberships::Column::PermissionsVersion,
            sea_orm::sea_query::Expr::col(
                med_tracker::models::_entities::household_memberships::Column::PermissionsVersion,
            )
            .add(1),
        )
        .filter(
            med_tracker::models::_entities::household_memberships::Column::HouseholdId
                .eq(fixture_household_id),
        )
        .filter(
            med_tracker::models::_entities::household_memberships::Column::PersonId
                .eq(fixture_minor_id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let tenant = begin(&app.fixture.runtime, &scope, None).await.unwrap();
    let remaining = reports::accessible_people(&tenant, PersonAccess::View)
        .await
        .unwrap();
    assert_eq!(remaining.len(), 2);
    assert!(!remaining.iter().any(|person| person.id == fixture_minor_id));
    tenant.rollback().await.unwrap();
    app.close().await;
}

#[tokio::test]
async fn gp_history_displays_takes_in_the_account_time_zone() {
    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_medication_id = app.fixture.medication.id;
    med_tracker::models::_entities::accounts::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::accounts::Column::Preferences,
            sea_orm::sea_query::Expr::value({
                let mut preferences = app.fixture.account.preferences.clone();
                preferences["time_zone"] = serde_json::json!("Europe/London");
                preferences
            }),
        )
        .filter(med_tracker::models::_entities::accounts::Column::Id.eq(fixture_account_id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let _schedules_83999 = med_tracker::models::_entities::schedules::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        person_id: sea_orm::Set(fixture_person_id),
        medication_id: sea_orm::Set(fixture_medication_id),
        active: sea_orm::Set(true),
        start_date: sea_orm::Set(Some("2026-01-01".parse::<chrono::NaiveDate>().unwrap())),
        end_date: sea_orm::Set(Some("2026-12-31".parse::<chrono::NaiveDate>().unwrap())),
        dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        schedule_type: sea_orm::Set(0),
        schedule_config: sea_orm::Set(serde_json::from_str::<serde_json::Value>("{}").unwrap()),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _medication_takes_84999 = med_tracker::models::_entities::medication_takes::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        schedule_id: sea_orm::Set(Some(_schedules_83999.id)),
        dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        taken_at: sea_orm::Set(Some(
            "2026-06-30T23:30:00"
                .parse::<chrono::NaiveDateTime>()
                .unwrap(),
        )),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let token = app.token().await;
    let response = app.client.get(format!("{}/api/v1/households/{fixture_household_id}/reports/health_history?person_id={fixture_person_id}&start_date=2026-07-01&end_date=2026-07-01&include_medication_takes=1", app.origin)).bearer_auth(&token).send().await.unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(
        body["data"]["medication_takes"][0]["taken_at"],
        "2026-07-01T00:30:00+01:00"
    );
    app.close().await;
}

#[tokio::test]
// Whole-run exclusive: This case changes the shared reference catalogue and removes only the rows it inserted.
async fn report_json_payloads_expose_only_the_documented_public_keys() {
    use sha2::Digest;
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_medication_id = app.fixture.medication.id;
    let _medication_review_evidence_records_90001 =
        med_tracker::models::_entities::medication_review_evidence_records::ActiveModel {
            source_name: sea_orm::Set("Synthetic reference".to_owned()),
            source_record_id: sea_orm::Set("report-1".to_owned()),
            source_url: sea_orm::Set("https://example.test/evidence/1".to_owned()),
            product_name: sea_orm::Set("Synthetic tablets".to_owned()),
            label_section: sea_orm::Set("Warnings".to_owned()),
            evidence_text: sea_orm::Set("Representative clinical review evidence.".to_owned()),
            retrieved_on: sea_orm::Set("2026-01-01".parse::<chrono::NaiveDate>().unwrap()),
            risk_level: sea_orm::Set("moderate".to_owned()),
            match_confidence: sea_orm::Set("high".to_owned()),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let _medication_review_prompts_91001 =
        med_tracker::models::_entities::medication_review_prompts::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            person_id: sea_orm::Set(fixture_person_id),
            primary_medication_id: sea_orm::Set(fixture_medication_id),
            interacting_medication_id: sea_orm::Set(fixture_medication_id),
            evidence_record_id: sea_orm::Set(_medication_review_evidence_records_90001.id),
            primary_medication_name: sea_orm::Set("Synthetic tablets".to_owned()),
            interacting_medication_name: sea_orm::Set("Synthetic tablets".to_owned()),
            evidence_source_name: sea_orm::Set("Synthetic reference".to_owned()),
            evidence_source_url: sea_orm::Set("https://example.test/evidence/1".to_owned()),
            evidence_source_version: sea_orm::Set("1".to_owned()),
            evidence_source_checked_on: sea_orm::Set(
                "2026-01-01".parse::<chrono::NaiveDate>().unwrap(),
            ),
            evidence_source_effective_on: sea_orm::Set(
                "2026-01-01".parse::<chrono::NaiveDate>().unwrap(),
            ),
            evidence_text: sea_orm::Set("Representative clinical review evidence.".to_owned()),
            match_confidence: sea_orm::Set("high".to_owned()),
            match_reason: sea_orm::Set("Clinical review required".to_owned()),
            match_type: sea_orm::Set("ingredient".to_owned()),
            matched_term: sea_orm::Set("synthetic".to_owned()),
            risk_level: sea_orm::Set("moderate".to_owned()),
            source_instruction: sea_orm::Set("unclassified".to_owned()),
            status: sea_orm::Set("needs_review".to_owned()),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let _schedules_83999 = med_tracker::models::_entities::schedules::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        person_id: sea_orm::Set(fixture_person_id),
        medication_id: sea_orm::Set(fixture_medication_id),
        active: sea_orm::Set(true),
        start_date: sea_orm::Set(Some("2026-01-01".parse::<chrono::NaiveDate>().unwrap())),
        end_date: sea_orm::Set(Some("2026-12-31".parse::<chrono::NaiveDate>().unwrap())),
        dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        schedule_type: sea_orm::Set(0),
        schedule_config: sea_orm::Set(serde_json::from_str::<serde_json::Value>("{}").unwrap()),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _medication_takes_84999 = med_tracker::models::_entities::medication_takes::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        schedule_id: sea_orm::Set(Some(_schedules_83999.id)),
        dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        taken_at: sea_orm::Set(Some(
            "2026-07-02T09:00:00"
                .parse::<chrono::NaiveDateTime>()
                .unwrap(),
        )),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let token = app.token().await;
    let health = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/reports/health_history?person_id={fixture_person_id}&start_date=2026-07-01&end_date=2026-07-07&include_medication_takes=1",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(health.status().as_u16(), 200);
    let health: Value = health.json().await.unwrap();
    let mut keys: Vec<&str> = health["data"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "chronology",
            "current_medicines",
            "end_date",
            "generated_at",
            "medication_takes",
            "person",
            "start_date"
        ]
    );
    let take = &health["data"]["medication_takes"][0];
    let mut take_keys: Vec<&str> = take
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    take_keys.sort_unstable();
    assert_eq!(
        take_keys,
        [
            "dose_amount",
            "dose_unit",
            "location_name",
            "medication_name",
            "source_type",
            "taken_at"
        ]
    );
    assert!(
        chrono::DateTime::parse_from_rfc3339(take["taken_at"].as_str().unwrap()).is_ok(),
        "taken_at must be RFC3339: {take}"
    );
    let reviews = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/reports/medication_reviews?person_id={fixture_person_id}",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(reviews.status().as_u16(), 200);
    let reviews: Value = reviews.json().await.unwrap();
    let mut keys: Vec<&str> = reviews["data"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["generated_at", "person", "prompts"]);
    let prompt = &reviews["data"]["prompts"][0];
    assert!(prompt.get("person_name").is_none(), "{prompt}");
    let etag = prompt["etag"].as_str().unwrap_or_default();
    let prompt_id: i64 = prompt["id"].as_str().unwrap().parse().unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(QueryExpr::col("updated_at"))
                .from("medication_review_prompts")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("id").eq(QueryExpr::val(prompt_id))),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let updated_at: chrono::NaiveDateTime = row.try_get("", "updated_at").unwrap();
    let canonical = format!(
        "MedicationReviewPrompt:{}:{}.{:06}",
        prompt_id,
        updated_at.and_utc().timestamp(),
        updated_at.and_utc().timestamp_subsec_micros()
    );
    let expected = format!(
        "\"{}\"",
        hex::encode(sha2::Sha256::digest(canonical.as_bytes()))
    );
    assert_eq!(
        etag, expected,
        "report prompt etag must equal the canonical sync tag"
    );
    {
        let prompts = med_tracker::models::_entities::medication_review_prompts::Entity::find()
            .filter(
                med_tracker::models::_entities::medication_review_prompts::Column::EvidenceRecordId
                    .is_in([_medication_review_evidence_records_90001.id]),
            )
            .all(&app.fixture.admin)
            .await
            .unwrap();
        let prompt_ids: Vec<i64> = prompts.iter().map(|prompt| prompt.id).collect();
        if !prompt_ids.is_empty() {
            med_tracker::models::_entities::medication_review_prompts::Entity::delete_many()
                .filter(
                    med_tracker::models::_entities::medication_review_prompts::Column::Id
                        .is_in(prompt_ids),
                )
                .filter(
                    med_tracker::models::_entities::medication_review_prompts::Column::HouseholdId
                        .eq(app.fixture.household.id),
                )
                .exec(&app.fixture.admin)
                .await
                .unwrap();
        }
    }
    med_tracker::models::_entities::medication_review_evidence_records::Entity::delete_by_id(
        _medication_review_evidence_records_90001.id,
    )
    .exec(&app.fixture.admin)
    .await
    .unwrap();
    app.close().await;
}

#[tokio::test]
async fn ordinary_report_compares_persisted_scheduled_at_as_utc_instants() {
    use med_tracker::models::{
        access::{Actor, HouseholdScope, PersonAccess, begin},
        care::reports,
    };
    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_medication_id = app.fixture.medication.id;
    let _schedules_83999 = med_tracker::models::_entities::schedules::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        person_id: sea_orm::Set(fixture_person_id),
        medication_id: sea_orm::Set(fixture_medication_id),
        active: sea_orm::Set(true),
        start_date: sea_orm::Set(Some("2026-01-01".parse::<chrono::NaiveDate>().unwrap())),
        end_date: sea_orm::Set(Some("2026-12-31".parse::<chrono::NaiveDate>().unwrap())),
        dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        schedule_type: sea_orm::Set(0),
        schedule_config: sea_orm::Set(serde_json::from_str::<serde_json::Value>("{}").unwrap()),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _medication_dose_occurrences_85999 =
        med_tracker::models::_entities::medication_dose_occurrences::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            schedule_id: sea_orm::Set(Some(_schedules_83999.id)),
            window_starts_on: sea_orm::Set("2026-07-01".parse::<chrono::NaiveDate>().unwrap()),
            window_ends_on: sea_orm::Set(Some("2026-07-01".parse::<chrono::NaiveDate>().unwrap())),
            position: sea_orm::Set(1),
            scheduled_at: sea_orm::Set(Some(
                "2026-07-01T10:30:00"
                    .parse::<chrono::NaiveDateTime>()
                    .unwrap(),
            )),
            outcome: sea_orm::Set("open".to_owned()),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let scope = HouseholdScope {
        actor: Actor {
            account_id: fixture_account_id,
        },
        household_id: fixture_household_id,
        request_id: "synthetic-utc-missed".to_owned(),
    };
    let tenant = begin(&app.fixture.runtime, &scope, None).await.unwrap();
    let people = reports::accessible_people(&tenant, PersonAccess::View)
        .await
        .unwrap();
    let day = chrono::NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
    let london = reports::ordinary_history(
        &tenant,
        &people,
        day,
        day,
        chrono::DateTime::parse_from_rfc3339("2026-07-01T10:00:00Z")
            .unwrap()
            .to_utc(),
        chrono_tz::Europe::London,
    )
    .await
    .map(|report| serde_json::to_value(report).unwrap())
    .unwrap();
    assert_eq!(
        london["daily_outcomes"][0]["date"], "2026-07-01",
        "{london}"
    );
    assert_eq!(
        london["daily_outcomes"][0]["unexplained_missed"], 0,
        "10:30 UTC is still ahead at 10:00 UTC / 11:00 BST: {london}"
    );
    let new_york = reports::ordinary_history(
        &tenant,
        &people,
        day,
        day,
        chrono::DateTime::parse_from_rfc3339("2026-07-01T12:00:00Z")
            .unwrap()
            .to_utc(),
        chrono_tz::America::New_York,
    )
    .await
    .map(|report| serde_json::to_value(report).unwrap())
    .unwrap();
    assert_eq!(
        new_york["daily_outcomes"][0]["unexplained_missed"], 1,
        "10:30 UTC is already past at 12:00 UTC / 08:00 EDT: {new_york}"
    );
    med_tracker::models::_entities::medication_dose_occurrences::Entity::update_many().col_expr(med_tracker::models::_entities::medication_dose_occurrences::Column::Outcome, sea_orm::sea_query::Expr::value("not_taken")).col_expr(med_tracker::models::_entities::medication_dose_occurrences::Column::Reason, sea_orm::sea_query::Expr::value(Some("other"))).col_expr(med_tracker::models::_entities::medication_dose_occurrences::Column::ResolvedAt, sea_orm::sea_query::Expr::value(Some("2026-07-01T11:00:00".parse::<chrono::NaiveDateTime>().unwrap()))).col_expr(med_tracker::models::_entities::medication_dose_occurrences::Column::ResolvedByMembershipId, sea_orm::sea_query::Expr::value(Some(app.fixture.membership.id))).col_expr(med_tracker::models::_entities::medication_dose_occurrences::Column::UpdatedAt, sea_orm::sea_query::Expr::value(chrono::Utc::now().naive_utc())).filter(med_tracker::models::_entities::medication_dose_occurrences::Column::Id.eq(_medication_dose_occurrences_85999.id)).filter(med_tracker::models::_entities::medication_dose_occurrences::Column::HouseholdId.eq(fixture_household_id)).exec(tenant.transaction()).await.unwrap();
    let london_resolved = reports::ordinary_history(
        &tenant,
        &people,
        day,
        day,
        chrono::DateTime::parse_from_rfc3339("2026-07-01T12:00:00Z")
            .unwrap()
            .to_utc(),
        chrono_tz::Europe::London,
    )
    .await
    .map(|report| serde_json::to_value(report).unwrap())
    .unwrap();
    assert_eq!(
        london_resolved["not_taken_outcomes"][0]["scheduled_at"], "2026-07-01 11:30",
        "10:30 UTC displays as 11:30 BST: {london_resolved}"
    );
    let new_york_resolved = reports::ordinary_history(
        &tenant,
        &people,
        day,
        day,
        chrono::DateTime::parse_from_rfc3339("2026-07-01T12:00:00Z")
            .unwrap()
            .to_utc(),
        chrono_tz::America::New_York,
    )
    .await
    .map(|report| serde_json::to_value(report).unwrap())
    .unwrap();
    assert_eq!(
        new_york_resolved["not_taken_outcomes"][0]["scheduled_at"], "2026-07-01 06:30",
        "10:30 UTC displays as 06:30 EDT: {new_york_resolved}"
    );
    tenant.rollback().await.unwrap();
    app.close().await;
}

#[tokio::test]
async fn ordinary_report_projects_and_attributes_takes_in_the_configured_zone() {
    use med_tracker::models::{
        access::{Actor, HouseholdScope, PersonAccess, begin},
        care::reports,
    };
    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_medication_id = app.fixture.medication.id;
    let _schedules_83999 = med_tracker::models::_entities::schedules::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        person_id: sea_orm::Set(fixture_person_id),
        medication_id: sea_orm::Set(fixture_medication_id),
        active: sea_orm::Set(true),
        start_date: sea_orm::Set(Some("2026-01-01".parse::<chrono::NaiveDate>().unwrap())),
        end_date: sea_orm::Set(Some("2026-12-31".parse::<chrono::NaiveDate>().unwrap())),
        dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        schedule_type: sea_orm::Set(0),
        schedule_config: sea_orm::Set(
            serde_json::from_str::<serde_json::Value>("{\"times\":[\"22:00\"]}").unwrap(),
        ),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _medication_takes_84999 = med_tracker::models::_entities::medication_takes::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        schedule_id: sea_orm::Set(Some(_schedules_83999.id)),
        dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        taken_at: sea_orm::Set(Some(
            "2026-03-09T03:30:00"
                .parse::<chrono::NaiveDateTime>()
                .unwrap(),
        )),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let scope = HouseholdScope {
        actor: Actor {
            account_id: fixture_account_id,
        },
        household_id: fixture_household_id,
        request_id: "synthetic-zone-projection".to_owned(),
    };
    let tenant = begin(&app.fixture.runtime, &scope, None).await.unwrap();
    let people = reports::accessible_people(&tenant, PersonAccess::View)
        .await
        .unwrap();
    let report = reports::ordinary_history(
        &tenant,
        &people,
        chrono::NaiveDate::from_ymd_opt(2026, 3, 7).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2026, 3, 9).unwrap(),
        chrono::DateTime::parse_from_rfc3339("2026-03-10T12:00:00Z")
            .unwrap()
            .to_utc(),
        chrono_tz::America::New_York,
    )
    .await
    .map(|report| serde_json::to_value(report).unwrap())
    .unwrap();
    let dst_sunday = report["daily_outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["date"] == "2026-03-08")
        .unwrap();
    assert_eq!(dst_sunday["expected"], 1, "{report}");
    assert_eq!(dst_sunday["actual"], 1, "{report}");
    assert_eq!(
        dst_sunday["unexplained_missed"], 0,
        "2026-03-09 03:30 UTC is 2026-03-08 23:30 EDT and satisfies the 22:00 dose: {report}"
    );
    let monday = report["daily_outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["date"] == "2026-03-09")
        .unwrap();
    assert_eq!(monday["expected"], 1, "{report}");
    assert_eq!(monday["actual"], 0, "{report}");
    assert_eq!(monday["unexplained_missed"], 1, "{report}");
    tenant.rollback().await.unwrap();
    app.close().await;
}

#[tokio::test]
async fn ordinary_report_forecasts_each_taper_step_at_its_own_dose() {
    use med_tracker::models::{
        access::{Actor, HouseholdScope, PersonAccess, begin},
        care::reports,
    };
    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_location_id = app.fixture.location.id;
    let fixture_medication_id = app.fixture.medication.id;
    let _medications_80002 = med_tracker::models::_entities::medications::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        location_id: sea_orm::Set(fixture_location_id),
        name: sea_orm::Set(Some("Second synthetic".to_owned())),
        current_supply: sea_orm::Set(Some("500".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_amount: sea_orm::Set(Some(1.0)),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _schedules_83999 = med_tracker::models::_entities::schedules::ActiveModel {
household_id: sea_orm::Set(fixture_household_id),
person_id: sea_orm::Set(fixture_person_id),
medication_id: sea_orm::Set(fixture_medication_id),
active: sea_orm::Set(true),
start_date: sea_orm::Set(Some("2026-07-01".parse::<chrono::NaiveDate>().unwrap())),
end_date: sea_orm::Set(Some("2026-12-31".parse::<chrono::NaiveDate>().unwrap())),
dose_amount: sea_orm::Set(Some("4".parse::<sea_orm::prelude::Decimal>().unwrap())),
dose_unit: sea_orm::Set(Some("tablet".to_owned())),
schedule_type: sea_orm::Set(5),
schedule_config: sea_orm::Set(serde_json::from_str::<serde_json::Value>("{\"taper_steps\":[{\"start_date\":\"2026-07-01\",\"end_date\":\"2026-07-10\",\"amount\":\"4\",\"unit\":\"tablet\",\"times\":[\"08:00\"]},{\"start_date\":\"2026-07-11\",\"end_date\":\"2026-07-20\",\"amount\":1,\"unit\":\"tablet\",\"times\":[\"08:00\",\"20:00\"]}]}").unwrap()),
created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
..Default::default()
}.insert(&app.fixture.admin).await.unwrap();
    let _schedules_83998 = med_tracker::models::_entities::schedules::ActiveModel {
household_id: sea_orm::Set(fixture_household_id),
person_id: sea_orm::Set(fixture_person_id),
medication_id: sea_orm::Set(_medications_80002.id),
active: sea_orm::Set(true),
start_date: sea_orm::Set(Some("2026-07-01".parse::<chrono::NaiveDate>().unwrap())),
end_date: sea_orm::Set(Some("2026-12-31".parse::<chrono::NaiveDate>().unwrap())),
dose_amount: sea_orm::Set(Some("4".parse::<sea_orm::prelude::Decimal>().unwrap())),
dose_unit: sea_orm::Set(Some("tablet".to_owned())),
schedule_type: sea_orm::Set(5),
schedule_config: sea_orm::Set(serde_json::from_str::<serde_json::Value>("{\"taper_steps\":[{\"start_date\":\"2026-07-01\",\"end_date\":\"2026-07-10\",\"amount\":\"4\",\"unit\":\"tablet\",\"times\":[\"08:00\"]},{\"start_date\":\"2026-07-11\",\"end_date\":\"2026-07-20\",\"amount\":1,\"unit\":\"tablet\",\"times\":[\"08:00\",\"20:00\"]}]}").unwrap()),
created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
..Default::default()
}.insert(&app.fixture.admin).await.unwrap();
    let scope = HouseholdScope {
        actor: Actor {
            account_id: fixture_account_id,
        },
        household_id: fixture_household_id,
        request_id: "synthetic-taper-forecast".to_owned(),
    };
    let tenant = begin(&app.fixture.runtime, &scope, None).await.unwrap();
    let people = reports::accessible_people(&tenant, PersonAccess::View)
        .await
        .unwrap();
    let report = reports::ordinary_history(
        &tenant,
        &people,
        chrono::NaiveDate::from_ymd_opt(2026, 7, 6).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2026, 7, 6).unwrap(),
        chrono::DateTime::parse_from_rfc3339("2026-07-06T12:00:00Z")
            .unwrap()
            .to_utc(),
        chrono_tz::UTC,
    )
    .await
    .map(|report| serde_json::to_value(report).unwrap())
    .unwrap();
    let alerts = report["inventory_alerts"].as_array().unwrap();
    assert_eq!(alerts.len(), 1, "500 units covers the taper: {report}");
    let alert = &alerts[0];
    assert_eq!(alert["medication_name"], "Synthetic tablets");
    assert_eq!(
        alert["days_left"], 7,
        "forecast = 5 days x 4 tablets + 10 days x 2 doses x 1 tablet = 40 over 31 days: {report}"
    );
    assert_eq!(alert["doses_left"], 10.0);
    assert_eq!(alert["low_stock"], false);
    tenant.rollback().await.unwrap();
    app.close().await;
}

#[tokio::test]
async fn ordinary_report_compliance_includes_cycle_expected_and_taken() {
    use med_tracker::models::{
        access::{Actor, HouseholdScope, PersonAccess, begin},
        care::reports,
    };
    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    med_tracker::models::_entities::person_medications::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::person_medications::Column::AdministrationKind,
            sea_orm::sea_query::Expr::value(0),
        )
        .col_expr(
            med_tracker::models::_entities::person_medications::Column::DoseCycle,
            sea_orm::sea_query::Expr::value(Some(1)),
        )
        .col_expr(
            med_tracker::models::_entities::person_medications::Column::MaxDailyDoses,
            sea_orm::sea_query::Expr::value(Some(1)),
        )
        .col_expr(
            med_tracker::models::_entities::person_medications::Column::Active,
            sea_orm::sea_query::Expr::value(true),
        )
        .col_expr(
            med_tracker::models::_entities::person_medications::Column::CreatedAt,
            sea_orm::sea_query::Expr::value(
                "2026-01-01T00:00:00"
                    .parse::<chrono::NaiveDateTime>()
                    .unwrap(),
            ),
        )
        .filter(
            med_tracker::models::_entities::person_medications::Column::Id
                .eq(fixture_assignment_id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let scope = HouseholdScope {
        actor: Actor {
            account_id: fixture_account_id,
        },
        household_id: fixture_household_id,
        request_id: "synthetic-cycle-summary".to_owned(),
    };
    let tenant = begin(&app.fixture.runtime, &scope, None).await.unwrap();
    let people = reports::accessible_people(&tenant, PersonAccess::View)
        .await
        .unwrap();
    let report = reports::ordinary_history(
        &tenant,
        &people,
        chrono::NaiveDate::from_ymd_opt(2026, 7, 6).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2026, 7, 12).unwrap(),
        chrono::DateTime::parse_from_rfc3339("2026-07-13T12:00:00Z")
            .unwrap()
            .to_utc(),
        chrono_tz::Europe::London,
    )
    .await
    .map(|report| serde_json::to_value(report).unwrap())
    .unwrap();
    assert_eq!(report["cycle_summaries"][0]["expected"], 1);
    assert_eq!(report["summary"]["expected"], 1);
    assert_eq!(report["summary"]["actual"], 0);
    assert_eq!(report["summary"]["compliance"], 0);
    tenant.rollback().await.unwrap();
    app.close().await;
}

#[tokio::test]
async fn report_projection_does_not_suppress_a_take_linked_to_another_source() {
    use med_tracker::models::{
        access::{Actor, HouseholdScope, PersonAccess, begin},
        care::reports,
    };
    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_medication_id = app.fixture.medication.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    med_tracker::models::_entities::person_medications::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::person_medications::Column::AdministrationKind,
            sea_orm::sea_query::Expr::value(0),
        )
        .col_expr(
            med_tracker::models::_entities::person_medications::Column::DoseCycle,
            sea_orm::sea_query::Expr::value(Some(1)),
        )
        .col_expr(
            med_tracker::models::_entities::person_medications::Column::MaxDailyDoses,
            sea_orm::sea_query::Expr::value(Some(1)),
        )
        .col_expr(
            med_tracker::models::_entities::person_medications::Column::Active,
            sea_orm::sea_query::Expr::value(true),
        )
        .col_expr(
            med_tracker::models::_entities::person_medications::Column::CreatedAt,
            sea_orm::sea_query::Expr::value(
                "2026-01-01T00:00:00"
                    .parse::<chrono::NaiveDateTime>()
                    .unwrap(),
            ),
        )
        .filter(
            med_tracker::models::_entities::person_medications::Column::Id
                .eq(fixture_assignment_id),
        )
        .filter(
            med_tracker::models::_entities::person_medications::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let _schedules_83999 = med_tracker::models::_entities::schedules::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        person_id: sea_orm::Set(fixture_person_id),
        medication_id: sea_orm::Set(fixture_medication_id),
        active: sea_orm::Set(false),
        start_date: sea_orm::Set(Some("2026-01-01".parse::<chrono::NaiveDate>().unwrap())),
        end_date: sea_orm::Set(Some("2026-12-31".parse::<chrono::NaiveDate>().unwrap())),
        dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        schedule_type: sea_orm::Set(0),
        schedule_config: sea_orm::Set(serde_json::from_str::<serde_json::Value>("{}").unwrap()),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _medication_takes_84999 = med_tracker::models::_entities::medication_takes::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        person_medication_id: sea_orm::Set(Some(fixture_assignment_id)),
        dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        taken_at: sea_orm::Set(Some(
            "2026-07-06T09:00:00"
                .parse::<chrono::NaiveDateTime>()
                .unwrap(),
        )),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _medication_dose_occurrences_85999 =
        med_tracker::models::_entities::medication_dose_occurrences::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            schedule_id: sea_orm::Set(Some(_schedules_83999.id)),
            medication_take_id: sea_orm::Set(Some(_medication_takes_84999.id)),
            window_starts_on: sea_orm::Set("2026-07-06".parse::<chrono::NaiveDate>().unwrap()),
            window_ends_on: sea_orm::Set(Some("2026-07-06".parse::<chrono::NaiveDate>().unwrap())),
            position: sea_orm::Set(1),
            outcome: sea_orm::Set("taken".to_owned()),
            resolved_at: sea_orm::Set(Some((chrono::Utc::now()).naive_utc())),
            resolved_by_membership_id: sea_orm::Set(Some(fixture_membership_id)),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let scope = HouseholdScope {
        actor: Actor {
            account_id: fixture_account_id,
        },
        household_id: fixture_household_id,
        request_id: "synthetic-cross-source-report".to_owned(),
    };
    let tenant = begin(&app.fixture.runtime, &scope, None).await.unwrap();
    let people = reports::accessible_people(&tenant, PersonAccess::View)
        .await
        .unwrap();
    let report = reports::ordinary_history(
        &tenant,
        &people,
        chrono::NaiveDate::from_ymd_opt(2026, 7, 6).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2026, 7, 12).unwrap(),
        chrono::DateTime::parse_from_rfc3339("2026-07-13T12:00:00Z")
            .unwrap()
            .to_utc(),
        chrono_tz::Europe::London,
    )
    .await
    .map(|report| serde_json::to_value(report).unwrap())
    .unwrap();
    let cycle = report["cycle_summaries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["source_id"] == fixture_assignment_id)
        .unwrap();
    assert_eq!(cycle["expected"], 1);
    assert_eq!(cycle["actual"], 1);
    tenant.rollback().await.unwrap();
    app.close().await;
}

#[tokio::test]
async fn gp_pdf_audit_identifies_the_person_dates_and_take_option() {
    use sea_orm::ConnectionTrait;
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let token = app.token().await;
    let response = app.client.get(format!("{}/api/v1/households/{fixture_household_id}/reports/health_history.pdf?person_id={fixture_person_id}&start_date=2026-07-01&end_date=2026-07-07&include_medication_takes=1", app.origin)).bearer_auth(&token).send().await.unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(QueryExpr::col("metadata"))
                .from("security_audit_events")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(
                            QueryExpr::col("event_type")
                                .eq(QueryExpr::val("health_history_report.downloaded")),
                        ),
                )
                .order_by_expr(QueryExpr::col("id"), sea_orm::sea_query::Order::Desc)
                .limit(1)
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let metadata: Value = row.try_get("", "metadata").unwrap();
    assert_eq!(metadata["person_id"], fixture_person_id);
    assert_eq!(metadata["start_date"], "2026-07-01");
    assert_eq!(metadata["end_date"], "2026-07-07");
    assert_eq!(metadata["include_medication_takes"], true);
    assert_eq!(metadata["format"], "pdf");
    assert_eq!(metadata["outcome"], "success");
    app.close().await;
}

#[tokio::test]
async fn gp_take_without_an_explicit_location_does_not_infer_one_from_the_medicine() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_medication_id = app.fixture.medication.id;
    let _schedules_83999 = med_tracker::models::_entities::schedules::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        person_id: sea_orm::Set(fixture_person_id),
        medication_id: sea_orm::Set(fixture_medication_id),
        active: sea_orm::Set(true),
        start_date: sea_orm::Set(Some("2026-01-01".parse::<chrono::NaiveDate>().unwrap())),
        end_date: sea_orm::Set(Some("2026-12-31".parse::<chrono::NaiveDate>().unwrap())),
        dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        schedule_type: sea_orm::Set(0),
        schedule_config: sea_orm::Set(serde_json::from_str::<serde_json::Value>("{}").unwrap()),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _medication_takes_84999 = med_tracker::models::_entities::medication_takes::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        schedule_id: sea_orm::Set(Some(_schedules_83999.id)),
        dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        taken_at: sea_orm::Set(Some(
            "2026-07-01T09:00:00"
                .parse::<chrono::NaiveDateTime>()
                .unwrap(),
        )),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let token = app.token().await;
    let response = app.client.get(format!("{}/api/v1/households/{fixture_household_id}/reports/health_history?person_id={fixture_person_id}&start_date=2026-07-01&end_date=2026-07-01&include_medication_takes=1", app.origin)).bearer_auth(&token).send().await.unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().await.unwrap();
    assert!(Value::is_null(
        &body["data"]["medication_takes"][0]["location_name"]
    ));
    app.close().await;
}

#[tokio::test]
async fn gp_current_medicines_exclude_legacy_schedules_without_dates() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_medication_id = app.fixture.medication.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    med_tracker::models::_entities::person_medications::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::person_medications::Column::Active,
            sea_orm::sea_query::Expr::value(false),
        )
        .col_expr(
            med_tracker::models::_entities::person_medications::Column::RetiredAt,
            sea_orm::sea_query::Expr::value(Some((chrono::Utc::now()).naive_utc())),
        )
        .filter(
            med_tracker::models::_entities::person_medications::Column::Id
                .eq(fixture_assignment_id),
        )
        .filter(
            med_tracker::models::_entities::person_medications::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let _schedules_83999 = med_tracker::models::_entities::schedules::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        person_id: sea_orm::Set(fixture_person_id),
        medication_id: sea_orm::Set(fixture_medication_id),
        active: sea_orm::Set(true),
        start_date: sea_orm::Set(None::<chrono::NaiveDate>),
        end_date: sea_orm::Set(None::<chrono::NaiveDate>),
        dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        schedule_type: sea_orm::Set(0),
        schedule_config: sea_orm::Set(serde_json::from_str::<serde_json::Value>("{}").unwrap()),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let token = app.token().await;
    let response = app.client.get(format!("{}/api/v1/households/{fixture_household_id}/reports/health_history?person_id={fixture_person_id}&start_date=2026-07-01&end_date=2026-07-07", app.origin)).bearer_auth(&token).send().await.unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["data"]["current_medicines"], serde_json::json!([]));
    app.close().await;
}

#[tokio::test]
async fn gp_current_medicines_use_retained_friendly_name_sorting() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_location_id = app.fixture.location.id;
    let fixture_medication_id = app.fixture.medication.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    med_tracker::models::_entities::person_medications::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::person_medications::Column::Active,
            sea_orm::sea_query::Expr::value(false),
        )
        .col_expr(
            med_tracker::models::_entities::person_medications::Column::RetiredAt,
            sea_orm::sea_query::Expr::value(Some((chrono::Utc::now()).naive_utc())),
        )
        .filter(
            med_tracker::models::_entities::person_medications::Column::Id
                .eq(fixture_assignment_id),
        )
        .filter(
            med_tracker::models::_entities::person_medications::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    med_tracker::models::_entities::medications::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::medications::Column::Name,
            sea_orm::sea_query::Expr::value(Some("Zulu".to_owned())),
        )
        .col_expr(
            med_tracker::models::_entities::medications::Column::FriendlyName,
            sea_orm::sea_query::Expr::value(None::<String>),
        )
        .filter(med_tracker::models::_entities::medications::Column::Id.eq(fixture_medication_id))
        .filter(
            med_tracker::models::_entities::medications::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let _medications_80002 = med_tracker::models::_entities::medications::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        location_id: sea_orm::Set(fixture_location_id),
        name: sea_orm::Set(Some("Alpha".to_owned())),
        friendly_name: sea_orm::Set(Some("Alpha".to_owned())),
        current_supply: sea_orm::Set(Some("10".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_amount: sea_orm::Set(Some(2.0)),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _schedules_83998 = med_tracker::models::_entities::schedules::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        person_id: sea_orm::Set(fixture_person_id),
        medication_id: sea_orm::Set(fixture_medication_id),
        active: sea_orm::Set(true),
        start_date: sea_orm::Set(Some(
            chrono::Utc::now().date_naive() - chrono::Duration::days(1),
        )),
        end_date: sea_orm::Set(Some(
            chrono::Utc::now().date_naive() + chrono::Duration::days(1),
        )),
        dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        schedule_type: sea_orm::Set(0),
        schedule_config: sea_orm::Set(serde_json::from_str::<serde_json::Value>("{}").unwrap()),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _schedules_83999 = med_tracker::models::_entities::schedules::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        person_id: sea_orm::Set(fixture_person_id),
        medication_id: sea_orm::Set(_medications_80002.id),
        active: sea_orm::Set(true),
        start_date: sea_orm::Set(Some(
            chrono::Utc::now().date_naive() - chrono::Duration::days(1),
        )),
        end_date: sea_orm::Set(Some(
            chrono::Utc::now().date_naive() + chrono::Duration::days(1),
        )),
        dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_unit: sea_orm::Set(Some("tablet".to_owned())),
        schedule_type: sea_orm::Set(0),
        schedule_config: sea_orm::Set(serde_json::from_str::<serde_json::Value>("{}").unwrap()),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let token = app.token().await;
    let response = app.client.get(format!("{}/api/v1/households/{fixture_household_id}/reports/health_history?person_id={fixture_person_id}&start_date=2026-07-01&end_date=2026-07-07", app.origin)).bearer_auth(&token).send().await.unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["data"]["current_medicines"][0]["name"], "Zulu");
    assert_eq!(body["data"]["current_medicines"][1]["name"], "Alpha");
    app.close().await;
}

#[tokio::test]
async fn ordinary_report_counts_daily_doses_from_each_taper_steps_times() {
    use med_tracker::models::{
        access::{Actor, HouseholdScope, PersonAccess, begin},
        care::reports,
    };
    let app = Application::new().await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_medication_id = app.fixture.medication.id;
    let _schedules_83999 = med_tracker::models::_entities::schedules::ActiveModel {
household_id: sea_orm::Set(fixture_household_id),
person_id: sea_orm::Set(fixture_person_id),
medication_id: sea_orm::Set(fixture_medication_id),
active: sea_orm::Set(true),
start_date: sea_orm::Set(Some("2026-07-01".parse::<chrono::NaiveDate>().unwrap())),
end_date: sea_orm::Set(Some("2026-12-31".parse::<chrono::NaiveDate>().unwrap())),
dose_amount: sea_orm::Set(Some("4".parse::<sea_orm::prelude::Decimal>().unwrap())),
dose_unit: sea_orm::Set(Some("tablet".to_owned())),
schedule_type: sea_orm::Set(5),
schedule_config: sea_orm::Set(serde_json::from_str::<serde_json::Value>("{\"taper_steps\":[{\"start_date\":\"2026-07-01\",\"end_date\":\"2026-07-10\",\"amount\":\"4\",\"unit\":\"tablet\",\"times\":[\"08:00\"]},{\"start_date\":\"2026-07-11\",\"end_date\":\"2026-07-20\",\"amount\":1,\"unit\":\"tablet\",\"times\":[\"08:00\",\"20:00\"]}]}").unwrap()),
created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
..Default::default()
}.insert(&app.fixture.admin).await.unwrap();
    let scope = HouseholdScope {
        actor: Actor {
            account_id: fixture_account_id,
        },
        household_id: fixture_household_id,
        request_id: "synthetic-taper-times".to_owned(),
    };
    let tenant = begin(&app.fixture.runtime, &scope, None).await.unwrap();
    let people = reports::accessible_people(&tenant, PersonAccess::View)
        .await
        .unwrap();
    let report = reports::ordinary_history(
        &tenant,
        &people,
        chrono::NaiveDate::from_ymd_opt(2026, 7, 8).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2026, 7, 12).unwrap(),
        chrono::DateTime::parse_from_rfc3339("2026-07-12T12:00:00Z")
            .unwrap()
            .to_utc(),
        chrono_tz::UTC,
    )
    .await
    .map(|report| serde_json::to_value(report).unwrap())
    .unwrap();
    let days = report["daily_outcomes"].as_array().unwrap();
    for (date, expected) in [
        ("2026-07-08", 1),
        ("2026-07-09", 1),
        ("2026-07-10", 1),
        ("2026-07-11", 2),
        ("2026-07-12", 2),
    ] {
        let day = days.iter().find(|row| row["date"] == date).unwrap();
        assert_eq!(
            day["expected"], expected,
            "taper step times must set the daily dose count: {report}"
        );
    }
    tenant.rollback().await.unwrap();
    app.close().await;
}

#[tokio::test]
// Whole-run exclusive: This case changes the shared reference catalogue and removes only the rows it inserted.
async fn medication_review_pdf_commits_download_audit_outside_the_snapshot_transaction() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_location_id = app.fixture.location.id;
    let token = app.token().await;
    let _medications_80003 = med_tracker::models::_entities::medications::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        location_id: sea_orm::Set(fixture_location_id),
        name: sea_orm::Set(Some("Synthetic capsules".to_owned())),
        current_supply: sea_orm::Set(Some("10".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_amount: sea_orm::Set(Some(1.0)),
        dose_unit: sea_orm::Set(Some("capsule".to_owned())),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _person_medications_81003 =
        med_tracker::models::_entities::person_medications::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            person_id: sea_orm::Set(fixture_person_id),
            medication_id: sea_orm::Set(_medications_80003.id),
            dose_amount: sea_orm::Set(Some("1".parse::<sea_orm::prelude::Decimal>().unwrap())),
            dose_unit: sea_orm::Set(Some("capsule".to_owned())),
            position: sea_orm::Set(1),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let _medication_review_evidence_records_90002 =
        med_tracker::models::_entities::medication_review_evidence_records::ActiveModel {
            source_name: sea_orm::Set("Synthetic reference".to_owned()),
            source_record_id: sea_orm::Set("report-2".to_owned()),
            source_url: sea_orm::Set("https://example.test/evidence/2".to_owned()),
            product_name: sea_orm::Set("Synthetic capsules".to_owned()),
            label_section: sea_orm::Set("Warnings".to_owned()),
            evidence_text: sea_orm::Set(
                "Synthetic capsules increase exposure to synthetic compounds.".to_owned(),
            ),
            retrieved_on: sea_orm::Set("2026-01-01".parse::<chrono::NaiveDate>().unwrap()),
            risk_level: sea_orm::Set("moderate".to_owned()),
            match_confidence: sea_orm::Set("high".to_owned()),
            match_status: sea_orm::Set("reviewed_pair".to_owned()),
            candidate_terms: sea_orm::Set(vec!["synthetic tablets".to_owned()]),
            interacting_terms: sea_orm::Set(vec!["synthetic capsules".to_owned()]),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/reports/medication_reviews.pdf?person_id={fixture_person_id}",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let request_id = response
        .headers()
        .get("x-request-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let bytes = response.bytes().await.unwrap();
    let prompt = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(QueryExpr::col("xmin").cast_as("text"), "xid")
                .from("medication_review_prompts")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(
                            QueryExpr::col("evidence_record_id")
                                .eq(QueryExpr::val(_medication_review_evidence_records_90002.id)),
                        ),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let prompt_xid: String = prompt.try_get("", "xid").unwrap();
    let audits = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(
                    QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(
                        sea_orm::sea_query::Asterisk,
                    ))),
                    "events",
                )
                .expr_as(
                    QueryExpr::expr(QueryFunc::count_distinct(
                        QueryExpr::col("xmin").cast_as("text"),
                    )),
                    "transactions",
                )
                .expr_as(
                    QueryExpr::expr(QueryFunc::min(QueryExpr::col("xmin").cast_as("text"))),
                    "xid",
                )
                .expr_as(
                    QueryExpr::expr(
                        QueryFunc::count(sea_orm::sea_query::Expr::col(
                            sea_orm::sea_query::Asterisk,
                        ))
                        .filter(
                            QueryExpr::col("event_type")
                                .eq(QueryExpr::val("medication_review_report.downloaded")),
                        ),
                    ),
                    "downloads",
                )
                .from("security_audit_events")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("request_id").eq(QueryExpr::val(request_id))),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let events: i64 = audits.try_get("", "events").unwrap();
    let transactions: i64 = audits.try_get("", "transactions").unwrap();
    let downloads: i64 = audits.try_get("", "downloads").unwrap();
    let audit_xid: String = audits.try_get("", "xid").unwrap();
    {
        let prompts = med_tracker::models::_entities::medication_review_prompts::Entity::find()
            .filter(
                med_tracker::models::_entities::medication_review_prompts::Column::EvidenceRecordId
                    .is_in([_medication_review_evidence_records_90002.id]),
            )
            .all(&app.fixture.admin)
            .await
            .unwrap();
        let prompt_ids: Vec<i64> = prompts.iter().map(|prompt| prompt.id).collect();
        if !prompt_ids.is_empty() {
            med_tracker::models::_entities::medication_review_prompts::Entity::delete_many()
                .filter(
                    med_tracker::models::_entities::medication_review_prompts::Column::Id
                        .is_in(prompt_ids),
                )
                .filter(
                    med_tracker::models::_entities::medication_review_prompts::Column::HouseholdId
                        .eq(app.fixture.household.id),
                )
                .exec(&app.fixture.admin)
                .await
                .unwrap();
        }
    }
    med_tracker::models::_entities::medication_review_evidence_records::Entity::delete_by_id(
        _medication_review_evidence_records_90002.id,
    )
    .exec(&app.fixture.admin)
    .await
    .unwrap();
    app.close().await;
    assert_eq!(status, 200);
    assert!(bytes.starts_with(b"%PDF-"));
    assert_eq!(events, 2);
    assert_eq!(downloads, 1);
    assert_eq!(
        transactions, 1,
        "download event and request audit must commit atomically"
    );
    assert_ne!(
        prompt_xid, audit_xid,
        "refresh writes must commit before the download audit transaction"
    );
}

#[tokio::test]
// Whole-run exclusive: PostgreSQL trigger functions, trigger installation and sequence privileges are unsupported by SeaORM Schema and SeaQuery Table/Query builders. The raw DDL is limited to this case and removed before closing.
async fn health_pdf_response_carries_no_bytes_or_audit_when_the_download_audit_fails() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("CREATE FUNCTION synthetic_report_audit_failure() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic report audit rejection'; END $$").await.unwrap();
    app.fixture.admin.execute_unprepared("CREATE TRIGGER synthetic_report_audit_failure BEFORE INSERT ON security_audit_events FOR EACH ROW WHEN (NEW.event_type='health_history_report.downloaded') EXECUTE FUNCTION synthetic_report_audit_failure()").await.unwrap();
    let response = app.client.get(format!("{}/api/v1/households/{fixture_household_id}/reports/health_history.pdf?person_id={fixture_person_id}&start_date=2026-01-01&end_date=2026-01-31", app.origin)).bearer_auth(&token).send().await.unwrap();
    let status = response.status().as_u16();
    let request_id = response
        .headers()
        .get("x-request-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let bytes = response.bytes().await.unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(
                    QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(
                        sea_orm::sea_query::Asterisk,
                    ))),
                    "events",
                )
                .from("security_audit_events")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("request_id").eq(QueryExpr::val(request_id))),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let events: i64 = row.try_get("", "events").unwrap();
    app.fixture
        .admin
        .execute_unprepared(
            "DROP TRIGGER IF EXISTS synthetic_report_audit_failure ON security_audit_events",
        )
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP FUNCTION IF EXISTS synthetic_report_audit_failure()")
        .await
        .unwrap();
    app.close().await;
    assert_eq!(status, 500);
    assert_ne!(content_type, "application/pdf");
    assert!(
        !bytes.starts_with(b"%PDF-"),
        "audit failure must not release PDF bytes"
    );
    assert_eq!(
        events, 0,
        "audit failure must leave no download or request audit"
    );
}

#[tokio::test]
// Whole-run exclusive: PostgreSQL trigger functions, trigger installation and sequence privileges are unsupported by SeaORM Schema and SeaQuery Table/Query builders. The raw DDL is limited to this case and removed before closing.
async fn medication_review_pdf_rechecks_person_access_after_rendering() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_location_id = app.fixture.location.id;
    let fixture_assignment_id = app.fixture.assignment.id;
    let token = app.token().await;
    app.fixture
        .admin
        .execute_unprepared("CREATE SEQUENCE synthetic_report_subject_revocation_reached")
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared("GRANT USAGE,SELECT ON SEQUENCE synthetic_report_subject_revocation_reached TO med_tracker_app").await.unwrap();
    med_tracker::models::_entities::person_medications::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::person_medications::Column::PersonId,
            sea_orm::sea_query::Expr::value(fixture_minor_id),
        )
        .filter(
            med_tracker::models::_entities::person_medications::Column::Id
                .eq(fixture_assignment_id),
        )
        .filter(
            med_tracker::models::_entities::person_medications::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let _medications_80003 = med_tracker::models::_entities::medications::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        location_id: sea_orm::Set(fixture_location_id),
        name: sea_orm::Set(Some("Synthetic capsules".to_owned())),
        current_supply: sea_orm::Set(Some("10".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_amount: sea_orm::Set(Some(1.0)),
        dose_unit: sea_orm::Set(Some("capsule".to_owned())),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _person_medications_81003 =
        med_tracker::models::_entities::person_medications::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            person_id: sea_orm::Set(fixture_minor_id),
            medication_id: sea_orm::Set(_medications_80003.id),
            dose_amount: sea_orm::Set(Some("1".parse::<sea_orm::prelude::Decimal>().unwrap())),
            dose_unit: sea_orm::Set(Some("capsule".to_owned())),
            position: sea_orm::Set(1),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let _medication_review_evidence_records_90002 =
        med_tracker::models::_entities::medication_review_evidence_records::ActiveModel {
            source_name: sea_orm::Set("Synthetic reference".to_owned()),
            source_record_id: sea_orm::Set("report-2".to_owned()),
            source_url: sea_orm::Set("https://example.test/evidence/2".to_owned()),
            product_name: sea_orm::Set("Synthetic capsules".to_owned()),
            label_section: sea_orm::Set("Warnings".to_owned()),
            evidence_text: sea_orm::Set(
                "Synthetic capsules increase exposure to synthetic compounds.".to_owned(),
            ),
            retrieved_on: sea_orm::Set("2026-01-01".parse::<chrono::NaiveDate>().unwrap()),
            risk_level: sea_orm::Set("moderate".to_owned()),
            match_confidence: sea_orm::Set("high".to_owned()),
            match_status: sea_orm::Set("reviewed_pair".to_owned()),
            candidate_terms: sea_orm::Set(vec!["synthetic tablets".to_owned()]),
            interacting_terms: sea_orm::Set(vec!["synthetic capsules".to_owned()]),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared(&format!("CREATE FUNCTION synthetic_revoke_subject_on_prompt() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('synthetic_report_subject_revocation_reached'); UPDATE household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE household_id={fixture_household_id} AND person_id={fixture_minor_id}; RETURN NEW; END $$")).await.unwrap();
    app.fixture.admin.execute_unprepared("CREATE TRIGGER synthetic_revoke_subject_on_prompt BEFORE INSERT ON medication_review_prompts FOR EACH ROW EXECUTE FUNCTION synthetic_revoke_subject_on_prompt()").await.unwrap();
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/reports/medication_reviews.pdf?person_id={fixture_minor_id}",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let request_id = response
        .headers()
        .get("x-request-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let bytes = response.bytes().await.unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(
                    QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(
                        sea_orm::sea_query::Asterisk,
                    ))),
                    "events",
                )
                .expr_as(
                    QueryExpr::expr(
                        QueryFunc::count(sea_orm::sea_query::Expr::col(
                            sea_orm::sea_query::Asterisk,
                        ))
                        .filter(
                            QueryExpr::col("event_type")
                                .eq(QueryExpr::val("medication_review_report.downloaded")),
                        ),
                    ),
                    "downloads",
                )
                .from("security_audit_events")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("request_id").eq(QueryExpr::val(request_id))),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let events: i64 = row.try_get("", "events").unwrap();
    let downloads: i64 = row.try_get("", "downloads").unwrap();
    let reached: bool = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(QueryExpr::col("is_called"), "reached")
                .from("synthetic_report_subject_revocation_reached")
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap()
        .try_get("", "reached")
        .unwrap();
    app.fixture.admin.execute_unprepared("DROP TRIGGER IF EXISTS synthetic_revoke_subject_on_prompt ON medication_review_prompts").await.unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP FUNCTION IF EXISTS synthetic_revoke_subject_on_prompt()")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP SEQUENCE IF EXISTS synthetic_report_subject_revocation_reached")
        .await
        .unwrap();
    {
        let prompts = med_tracker::models::_entities::medication_review_prompts::Entity::find()
            .filter(
                med_tracker::models::_entities::medication_review_prompts::Column::EvidenceRecordId
                    .is_in([_medication_review_evidence_records_90002.id]),
            )
            .all(&app.fixture.admin)
            .await
            .unwrap();
        let prompt_ids: Vec<i64> = prompts.iter().map(|prompt| prompt.id).collect();
        if !prompt_ids.is_empty() {
            med_tracker::models::_entities::medication_review_prompts::Entity::delete_many()
                .filter(
                    med_tracker::models::_entities::medication_review_prompts::Column::Id
                        .is_in(prompt_ids),
                )
                .filter(
                    med_tracker::models::_entities::medication_review_prompts::Column::HouseholdId
                        .eq(app.fixture.household.id),
                )
                .exec(&app.fixture.admin)
                .await
                .unwrap();
        }
    }
    med_tracker::models::_entities::medication_review_evidence_records::Entity::delete_by_id(
        _medication_review_evidence_records_90002.id,
    )
    .exec(&app.fixture.admin)
    .await
    .unwrap();
    app.close().await;
    assert!(
        reached,
        "The subject-participation revocation trigger must be reached during rendering"
    );
    assert_eq!(
        status, 404,
        "a subject participation revoked during the snapshot must deny the download"
    );
    assert!(
        !bytes.starts_with(b"%PDF-"),
        "a denied request must not release PDF bytes"
    );
    assert_eq!(events, 1, "the denied download still records an audit");
    assert_eq!(downloads, 0);
}

#[tokio::test]
// Whole-run exclusive: PostgreSQL trigger functions, trigger installation and sequence privileges are unsupported by SeaORM Schema and SeaQuery Table/Query builders. The raw DDL is limited to this case and removed before closing.
async fn medication_review_pdf_rechecks_actor_policy_after_rendering() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_location_id = app.fixture.location.id;
    let token = app.token().await;
    let _medications_80003 = med_tracker::models::_entities::medications::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        location_id: sea_orm::Set(fixture_location_id),
        name: sea_orm::Set(Some("Synthetic capsules".to_owned())),
        current_supply: sea_orm::Set(Some("10".parse::<sea_orm::prelude::Decimal>().unwrap())),
        dose_amount: sea_orm::Set(Some(1.0)),
        dose_unit: sea_orm::Set(Some("capsule".to_owned())),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _person_medications_81003 =
        med_tracker::models::_entities::person_medications::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            person_id: sea_orm::Set(fixture_person_id),
            medication_id: sea_orm::Set(_medications_80003.id),
            dose_amount: sea_orm::Set(Some("1".parse::<sea_orm::prelude::Decimal>().unwrap())),
            dose_unit: sea_orm::Set(Some("capsule".to_owned())),
            position: sea_orm::Set(1),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let _medication_review_evidence_records_90002 =
        med_tracker::models::_entities::medication_review_evidence_records::ActiveModel {
            source_name: sea_orm::Set("Synthetic reference".to_owned()),
            source_record_id: sea_orm::Set("report-2".to_owned()),
            source_url: sea_orm::Set("https://example.test/evidence/2".to_owned()),
            product_name: sea_orm::Set("Synthetic capsules".to_owned()),
            label_section: sea_orm::Set("Warnings".to_owned()),
            evidence_text: sea_orm::Set(
                "Synthetic capsules increase exposure to synthetic compounds.".to_owned(),
            ),
            retrieved_on: sea_orm::Set("2026-01-01".parse::<chrono::NaiveDate>().unwrap()),
            risk_level: sea_orm::Set("moderate".to_owned()),
            match_confidence: sea_orm::Set("high".to_owned()),
            match_status: sea_orm::Set("reviewed_pair".to_owned()),
            candidate_terms: sea_orm::Set(vec!["synthetic tablets".to_owned()]),
            interacting_terms: sea_orm::Set(vec!["synthetic capsules".to_owned()]),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared(&format!("CREATE FUNCTION synthetic_minor_actor_on_prompt() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN UPDATE people SET person_type=1,has_capacity=false,date_of_birth=NULL WHERE id=(SELECT person_id FROM household_memberships WHERE id={fixture_membership_id}); RETURN NEW; END $$")).await.unwrap();
    app.fixture.admin.execute_unprepared("CREATE TRIGGER synthetic_minor_actor_on_prompt BEFORE INSERT ON medication_review_prompts FOR EACH ROW EXECUTE FUNCTION synthetic_minor_actor_on_prompt()").await.unwrap();
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/reports/medication_reviews.pdf?person_id={fixture_person_id}",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let request_id = response
        .headers()
        .get("x-request-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let bytes = response.bytes().await.unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(
                    QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(
                        sea_orm::sea_query::Asterisk,
                    ))),
                    "events",
                )
                .expr_as(
                    QueryExpr::expr(
                        QueryFunc::count(sea_orm::sea_query::Expr::col(
                            sea_orm::sea_query::Asterisk,
                        ))
                        .filter(
                            QueryExpr::col("event_type")
                                .eq(QueryExpr::val("medication_review_report.downloaded")),
                        ),
                    ),
                    "downloads",
                )
                .from("security_audit_events")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("request_id").eq(QueryExpr::val(request_id))),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let events: i64 = row.try_get("", "events").unwrap();
    let downloads: i64 = row.try_get("", "downloads").unwrap();
    app.fixture
        .admin
        .execute_unprepared(
            "DROP TRIGGER IF EXISTS synthetic_minor_actor_on_prompt ON medication_review_prompts",
        )
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP FUNCTION IF EXISTS synthetic_minor_actor_on_prompt()")
        .await
        .unwrap();
    {
        let prompts = med_tracker::models::_entities::medication_review_prompts::Entity::find()
            .filter(
                med_tracker::models::_entities::medication_review_prompts::Column::EvidenceRecordId
                    .is_in([_medication_review_evidence_records_90002.id]),
            )
            .all(&app.fixture.admin)
            .await
            .unwrap();
        let prompt_ids: Vec<i64> = prompts.iter().map(|prompt| prompt.id).collect();
        if !prompt_ids.is_empty() {
            med_tracker::models::_entities::medication_review_prompts::Entity::delete_many()
                .filter(
                    med_tracker::models::_entities::medication_review_prompts::Column::Id
                        .is_in(prompt_ids),
                )
                .filter(
                    med_tracker::models::_entities::medication_review_prompts::Column::HouseholdId
                        .eq(app.fixture.household.id),
                )
                .exec(&app.fixture.admin)
                .await
                .unwrap();
        }
    }
    med_tracker::models::_entities::medication_review_evidence_records::Entity::delete_by_id(
        _medication_review_evidence_records_90002.id,
    )
    .exec(&app.fixture.admin)
    .await
    .unwrap();
    app.close().await;
    assert_eq!(
        status, 403,
        "an actor losing adult status during the snapshot must deny the download"
    );
    assert!(
        !bytes.starts_with(b"%PDF-"),
        "a denied request must not release PDF bytes"
    );
    assert_eq!(events, 1, "the denied download still records an audit");
    assert_eq!(downloads, 0);
}
