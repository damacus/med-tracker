use super::*;

#[tokio::test]
// Whole-run exclusive: This case changes the shared reference catalogue and removes only the rows it inserted.
async fn medication_lookup_preserves_related_trade_family_and_review_evidence() {
    crate::reference_fixture::clean_nhs_catalogue().await;
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_medication_id = app.fixture.medication.id;
    let fixture_slug = app.fixture.household.slug.clone();
    let seeded_catalogue = seed_catalog(&app).await;
    med_tracker::models::_entities::medications::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::medications::Column::Name,
            sea_orm::sea_query::Expr::value(Some("Warfarin".to_owned())),
        )
        .col_expr(
            med_tracker::models::_entities::medications::Column::DmdCode,
            sea_orm::sea_query::Expr::value(None::<String>),
        )
        .col_expr(
            med_tracker::models::_entities::medications::Column::Barcode,
            sea_orm::sea_query::Expr::value(Some("5016298210989".to_owned())),
        )
        .filter(med_tracker::models::_entities::medications::Column::Id.eq(fixture_medication_id))
        .filter(
            med_tracker::models::_entities::medications::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    med_tracker::models::_entities::nhs_dmd_barcodes::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::nhs_dmd_barcodes::Column::Display,
            sea_orm::sea_query::Expr::value("Aspirin tablets".to_owned()),
        )
        .col_expr(
            med_tracker::models::_entities::nhs_dmd_barcodes::Column::VmpName,
            sea_orm::sea_query::Expr::value(Some("Aspirin tablets".to_owned())),
        )
        .col_expr(
            med_tracker::models::_entities::nhs_dmd_barcodes::Column::AmpCode,
            sea_orm::sea_query::Expr::value(Some("AMP1".to_owned())),
        )
        .filter(
            med_tracker::models::_entities::nhs_dmd_barcodes::Column::Id.eq(seeded_catalogue.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let catalogue_row_0 = med_tracker::models::_entities::nhs_dmd_barcodes::ActiveModel {
        gtin: sea_orm::Set("5016298210989".to_owned()),
        code: sea_orm::Set("222".to_owned()),
        display: sea_orm::Set("Warfarin tablets".to_owned()),
        system: sea_orm::Set("https://dmd.nhs.uk".to_owned()),
        amp_code: sea_orm::Set(Some("AMP2".to_owned())),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _nhs_dmd_trade_family_groups_92001 =
        med_tracker::models::_entities::nhs_dmd_trade_family_groups::ActiveModel {
            code: sea_orm::Set("GROUP1".to_owned()),
            name: sea_orm::Set("Synthetic group".to_owned()),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let _nhs_dmd_trade_families_92001 =
        med_tracker::models::_entities::nhs_dmd_trade_families::ActiveModel {
            code: sea_orm::Set("FAMILY1".to_owned()),
            name: sea_orm::Set("Synthetic family".to_owned()),
            trade_family_group_id: sea_orm::Set(Some(_nhs_dmd_trade_family_groups_92001.id)),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let catalogue_row_1 = med_tracker::models::_entities::nhs_dmd_amp_trade_families::ActiveModel {
        amp_code: sea_orm::Set("AMP1".to_owned()),
        trade_family_id: sea_orm::Set(_nhs_dmd_trade_families_92001.id),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let catalogue_row_2 = med_tracker::models::_entities::nhs_dmd_amp_trade_families::ActiveModel {
        amp_code: sea_orm::Set("AMP2".to_owned()),
        trade_family_id: sea_orm::Set(_nhs_dmd_trade_families_92001.id),
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
            source_record_id: sea_orm::Set("lookup-1".to_owned()),
            source_url: sea_orm::Set("https://example.test/evidence/lookup".to_owned()),
            product_name: sea_orm::Set("Aspirin".to_owned()),
            label_section: sea_orm::Set("Warnings".to_owned()),
            evidence_text: sea_orm::Set("Avoid warfarin with aspirin.".to_owned()),
            retrieved_on: sea_orm::Set("2026-01-01".parse::<chrono::NaiveDate>().unwrap()),
            risk_level: sea_orm::Set("high".to_owned()),
            match_confidence: sea_orm::Set("high".to_owned()),
            match_status: sea_orm::Set("reviewed_pair".to_owned()),
            candidate_terms: sea_orm::Set(vec!["aspirin".to_owned()]),
            interacting_terms: sea_orm::Set(vec!["warfarin".to_owned()]),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let token = app.token().await;
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/medication_lookup?q=5000168511017",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body = response.json::<Value>().await.unwrap();
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
    med_tracker::models::_entities::nhs_dmd_barcodes::Entity::delete_by_id(seeded_catalogue.id)
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    med_tracker::models::_entities::medication_review_evidence_records::Entity::delete_by_id(
        _medication_review_evidence_records_90002.id,
    )
    .exec(&app.fixture.admin)
    .await
    .unwrap();
    med_tracker::models::_entities::nhs_dmd_amp_trade_families::Entity::delete_by_id(
        catalogue_row_2.id,
    )
    .exec(&app.fixture.admin)
    .await
    .unwrap();
    med_tracker::models::_entities::nhs_dmd_amp_trade_families::Entity::delete_by_id(
        catalogue_row_1.id,
    )
    .exec(&app.fixture.admin)
    .await
    .unwrap();
    med_tracker::models::_entities::nhs_dmd_trade_families::Entity::delete_by_id(
        _nhs_dmd_trade_families_92001.id,
    )
    .exec(&app.fixture.admin)
    .await
    .unwrap();
    med_tracker::models::_entities::nhs_dmd_trade_family_groups::Entity::delete_by_id(
        _nhs_dmd_trade_family_groups_92001.id,
    )
    .exec(&app.fixture.admin)
    .await
    .unwrap();
    med_tracker::models::_entities::nhs_dmd_barcodes::Entity::delete_by_id(catalogue_row_0.id)
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    app.close().await;
    assert_eq!(status, 200, "{body}");
    let result = &body["results"][0];
    assert_eq!(result["trade_family"]["code"], "FAMILY1", "{body}");
    assert_eq!(result["trade_family_group"]["code"], "GROUP1", "{body}");
    assert_eq!(
        result["related_medications"][0]["id"], fixture_medication_id,
        "{body}"
    );
    assert_eq!(
        result["related_medications"][0]["path"],
        format!("/households/{fixture_slug}/medications/{fixture_medication_id}"),
        "{body}"
    );
    assert!(
        result["related_medications"][0]
            .get("refill_path")
            .is_none(),
        "{body}"
    );
    assert_eq!(
        result["review_prompts"][0]["evidence_record_id"],
        _medication_review_evidence_records_90002.id,
        "{body}"
    );
    assert_eq!(
        result["review_prompts"][0]["interacting_medication_name"],
        "Warfarin"
    );
    assert!(result.get("existing_medication").is_none());
}

#[tokio::test]
// Whole-run exclusive: This case changes the shared reference catalogue and removes only the rows it inserted.
async fn medication_lookup_keeps_catalogue_results_when_review_evidence_is_unavailable() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let seeded_catalogue = seed_catalog(&app).await;
    {
        let rename = sea_orm::sea_query::Table::rename()
            .table(
                "medication_review_evidence_records",
                "unavailable_review_evidence_records",
            )
            .to_owned();
        app.fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&rename))
            .await
            .unwrap();
    }
    let token = app.token().await;
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/medication_lookup?q=5000168511017",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body = response.json::<Value>().await.unwrap_or(Value::Null);
    {
        let rename = sea_orm::sea_query::Table::rename()
            .table(
                "unavailable_review_evidence_records",
                "medication_review_evidence_records",
            )
            .to_owned();
        app.fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&rename))
            .await
            .unwrap();
    }
    med_tracker::models::_entities::nhs_dmd_barcodes::Entity::delete_by_id(seeded_catalogue.id)
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    app.close().await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["results"][0]["code"], "123456", "{body}");
    assert_eq!(body["review_guidance"]["status"], "unavailable", "{body}");
}

#[tokio::test]
// Whole-run exclusive: This case changes the shared reference catalogue and removes only the rows it inserted.
async fn medication_lookup_matches_pack_names_without_confusing_strengths() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_location_id = app.fixture.location.id;
    let fixture_medication_id = app.fixture.medication.id;
    let fixture_slug = app.fixture.household.slug.clone();
    med_tracker::models::_entities::medications::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::medications::Column::Name,
            sea_orm::sea_query::Expr::value(Some("Metformin".to_owned())),
        )
        .col_expr(
            med_tracker::models::_entities::medications::Column::DoseAmount,
            sea_orm::sea_query::Expr::value(Some(500.0)),
        )
        .col_expr(
            med_tracker::models::_entities::medications::Column::DoseUnit,
            sea_orm::sea_query::Expr::value(Some("mg".to_owned())),
        )
        .col_expr(
            med_tracker::models::_entities::medications::Column::Barcode,
            sea_orm::sea_query::Expr::value(None::<String>),
        )
        .col_expr(
            med_tracker::models::_entities::medications::Column::DmdCode,
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
    let _medications_80003 = med_tracker::models::_entities::medications::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        location_id: sea_orm::Set(fixture_location_id),
        name: sea_orm::Set(Some("Metformin".to_owned())),
        dose_amount: sea_orm::Set(Some(1000.0)),
        dose_unit: sea_orm::Set(Some("mg".to_owned())),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let catalogue_row_0 = med_tracker::models::_entities::nhs_dmd_barcodes::ActiveModel {
        gtin: sea_orm::Set("5000168511017".to_owned()),
        code: sea_orm::Set("NEW".to_owned()),
        display: sea_orm::Set("Metformin 500mg 28 tablets (Synthetic manufacturer)".to_owned()),
        system: sea_orm::Set("https://dmd.nhs.uk".to_owned()),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let token = app.token().await;
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/medication_lookup?q=5000168511017",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body = response.json::<Value>().await.unwrap();
    med_tracker::models::_entities::nhs_dmd_barcodes::Entity::delete_by_id(catalogue_row_0.id)
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    app.close().await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body["results"][0]["existing_medication"]["id"], fixture_medication_id,
        "{body}"
    );
    assert_eq!(
        body["results"][0]["existing_medication"]["path"],
        format!("/households/{fixture_slug}/medications/{fixture_medication_id}"),
        "{body}"
    );
    assert_eq!(
        body["results"][0]["existing_medication"]["refill_path"],
        format!("/households/{fixture_slug}/medications/{fixture_medication_id}?refill=true"),
        "{body}"
    );
    assert_eq!(
        body["query"], "Metformin 500mg 28 tablets (Synthetic manufacturer)",
        "{body}"
    );
}

#[tokio::test]
// Whole-run exclusive: This case changes the shared reference catalogue and removes only the rows it inserted.
async fn medication_lookup_searches_names_case_insensitively_and_accepts_form_aliases() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let seeded_catalogue = seed_catalog(&app).await;
    let token = app.token().await;
    let mut observed = Vec::new();
    for query in ["synthetic", "SYNTHETIC", "SyNtHeTiC"] {
        let response = app
            .client
            .get(format!(
                "{}/api/v1/households/{fixture_household_id}/medication_lookup?q={query}&form=caplet",
                app.origin
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        observed.push((
            response.status().as_u16(),
            response.json::<Value>().await.unwrap(),
        ));
    }
    med_tracker::models::_entities::nhs_dmd_barcodes::Entity::delete_by_id(seeded_catalogue.id)
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    app.close().await;
    for (status, body) in observed {
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["results"][0]["code"], "123456", "{body}");
        assert!(body["results"][0]["match_reason"].is_null());
        assert_eq!(body["form"], "tablet", "{body}");
    }
}

#[tokio::test]
// Whole-run exclusive: This case changes the shared reference catalogue and removes only the rows it inserted.
async fn medication_lookup_normalizes_strength_and_preserves_distinct_gtins() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let catalogue_row_0 = med_tracker::models::_entities::nhs_dmd_barcodes::ActiveModel {
        gtin: sea_orm::Set("5000168511017".to_owned()),
        code: sea_orm::Set("STRENGTH".to_owned()),
        display: sea_orm::Set("Synthetic 1mg tablets".to_owned()),
        system: sea_orm::Set("https://dmd.nhs.uk".to_owned()),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let catalogue_row_1 = med_tracker::models::_entities::nhs_dmd_barcodes::ActiveModel {
        gtin: sea_orm::Set("5000168511018".to_owned()),
        code: sea_orm::Set("STRENGTH".to_owned()),
        display: sea_orm::Set("Synthetic 1mg tablets".to_owned()),
        system: sea_orm::Set("https://dmd.nhs.uk".to_owned()),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let token = app.token().await;
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/medication_lookup?q=Synthetic&strength=1000mcg&form=caplet",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body = response.json::<Value>().await.unwrap();
    med_tracker::models::_entities::nhs_dmd_barcodes::Entity::delete_by_id(catalogue_row_1.id)
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    med_tracker::models::_entities::nhs_dmd_barcodes::Entity::delete_by_id(catalogue_row_0.id)
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    app.close().await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["strength"], "1mg", "{body}");
    assert_eq!(body["form"], "tablet", "{body}");
    assert_eq!(body["results"].as_array().unwrap().len(), 2, "{body}");
    assert_ne!(
        body["results"][0]["barcode"], body["results"][1]["barcode"],
        "{body}"
    );
}

async fn seed_catalog(
    app: &Application,
) -> med_tracker::models::_entities::nhs_dmd_barcodes::Model {
    let fixture_medication_id = app.fixture.medication.id;
    let catalogue_row_0 = med_tracker::models::_entities::nhs_dmd_barcodes::ActiveModel {
        gtin: sea_orm::Set("05000168511017".to_owned()),
        code: sea_orm::Set("123456".to_owned()),
        display: sea_orm::Set("Synthetic tablets 28 tablets".to_owned()),
        vmp_name: sea_orm::Set(Some("Synthetic tablets".to_owned())),
        system: sea_orm::Set("https://dmd.nhs.uk".to_owned()),
        concept_class: sea_orm::Set(Some("AMPP".to_owned())),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    med_tracker::models::_entities::medications::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::medications::Column::DmdCode,
            sea_orm::sea_query::Expr::value(Some("123456".to_owned())),
        )
        .col_expr(
            med_tracker::models::_entities::medications::Column::DmdSystem,
            sea_orm::sea_query::Expr::value(Some("https://dmd.nhs.uk".to_owned())),
        )
        .filter(med_tracker::models::_entities::medications::Column::Id.eq(fixture_medication_id))
        .filter(
            med_tracker::models::_entities::medications::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    catalogue_row_0
}

#[tokio::test]
// Whole-run exclusive: This case changes the shared reference catalogue and removes only the rows it inserted.
async fn medication_lookup_resolves_imported_gtin_and_unique_visible_stock() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_medication_id = app.fixture.medication.id;
    let seeded_catalogue = seed_catalog(&app).await;
    let token = app.token().await;
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/medication_lookup?q=5000168511017",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body = response.json::<Value>().await.unwrap_or(Value::Null);
    med_tracker::models::_entities::nhs_dmd_barcodes::Entity::delete_by_id(seeded_catalogue.id)
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    app.close().await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body["barcode_resolution"],
        json!({"status":"resolved","source":"nhs_dmd"})
    );
    assert_eq!(body["results"][0]["code"], "123456");
    assert_eq!(
        body["results"][0]["existing_medication"]["id"],
        fixture_medication_id
    );
    assert_eq!(body["permissions"]["can_create"], true);
}

#[tokio::test]
// Whole-run exclusive: This case changes the shared reference catalogue and removes only the rows it inserted.
async fn medication_lookup_never_picks_an_ambiguous_stock_record() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_location_id = app.fixture.location.id;
    let seeded_catalogue = seed_catalog(&app).await;
    let _medications_80003 = med_tracker::models::_entities::medications::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        location_id: sea_orm::Set(fixture_location_id),
        name: sea_orm::Set(Some("Second cabinet tablets".to_owned())),
        dmd_code: sea_orm::Set(Some("123456".to_owned())),
        dmd_system: sea_orm::Set(Some("https://dmd.nhs.uk".to_owned())),
        current_supply: sea_orm::Set(Some("4".parse::<sea_orm::prelude::Decimal>().unwrap())),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let token = app.token().await;
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/medication_lookup?q=5000168511017",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body = response.json::<Value>().await.unwrap_or(Value::Null);
    med_tracker::models::_entities::nhs_dmd_barcodes::Entity::delete_by_id(seeded_catalogue.id)
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    app.close().await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["results"].as_array().unwrap().len(), 1);
    assert!(body["results"][0].get("existing_medication").is_none());
}

#[tokio::test]
async fn medication_lookup_rejects_unauthenticated_and_foreign_households() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let foreign_household = app.fixture.seed_foreign_household(0).await;
    let token = app.token().await;
    let anonymous = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/medication_lookup?q=5000168511017",
            app.origin
        ))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/{foreign_household_id}/medication_lookup?q=5000168511017",
            app.origin,
            foreign_household_id = foreign_household.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.close().await;
    assert_eq!(anonymous, 401);
    assert!(matches!(foreign, 403 | 404));
}

#[tokio::test]
// Whole-run exclusive: This case changes the shared reference catalogue and removes only the rows it inserted.
async fn medication_lookup_preserves_local_source_priority_and_manual_misses() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let seeded_catalogue = seed_catalog(&app).await;
    let catalogue_row_0 = med_tracker::models::_entities::barcode_catalog_entries::ActiveModel {
        gtin: sea_orm::Set("5000168511017".to_owned()),
        display: sea_orm::Set("Imported custom medicine".to_owned()),
        source: sea_orm::Set("household_import".to_owned()),
        code: sea_orm::Set(Some("CUSTOM".to_owned())),
        system: sea_orm::Set(Some("Local catalogue".to_owned())),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let token = app.token().await;
    let url = format!(
        "{}/api/v1/households/{fixture_household_id}/medication_lookup",
        app.origin
    );
    let first = app
        .client
        .get(format!("{url}?q=5000168511017"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let first_status = first.status().as_u16();
    let first_body = first.json::<Value>().await.unwrap_or(Value::Null);
    let missing = app
        .client
        .get(format!("{url}?q=9999999999999"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let missing_status = missing.status().as_u16();
    let missing_body = missing.json::<Value>().await.unwrap_or(Value::Null);
    let blank = app
        .client
        .get(&url)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let blank_status = blank.status().as_u16();
    let blank_body = blank.json::<Value>().await.unwrap_or(Value::Null);
    med_tracker::models::_entities::nhs_dmd_barcodes::Entity::delete_by_id(seeded_catalogue.id)
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    med_tracker::models::_entities::barcode_catalog_entries::Entity::delete_by_id(
        catalogue_row_0.id,
    )
    .exec(&app.fixture.admin)
    .await
    .unwrap();
    app.close().await;
    assert_eq!(
        (first_status, missing_status, blank_status),
        (200, 200, 200)
    );
    assert_eq!(
        first_body["results"][0]["display"],
        "Imported custom medicine"
    );
    assert_eq!(
        first_body["barcode_resolution"]["source"],
        "household_import"
    );
    assert_eq!(missing_body["results"], json!([]));
    assert_eq!(blank_body["results"], json!([]));
}

#[tokio::test]
async fn medication_lookup_preserves_curated_warnings_without_external_requests() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_household_id}/medication_lookup?q=5057753926137",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body = response.json::<Value>().await.unwrap_or(Value::Null);
    app.close().await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["barcode_resolution"]["source"], "curated");
    assert!(
        body["results"][0]["warnings"]
            .as_str()
            .unwrap()
            .contains("vitamin A")
    );
    assert_eq!(body["results"][0]["category"], "Vitamin");
}
