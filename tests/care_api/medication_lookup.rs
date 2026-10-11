use super::*;

#[tokio::test]
async fn medication_lookup_preserves_related_trade_family_and_review_evidence() {
    let app = Application::new().await;
    seed_catalog(&app).await;
    app.fixture.admin.execute_unprepared("UPDATE medications SET name='Warfarin',dmd_code=NULL,barcode='5016298210989' WHERE id=80001; UPDATE nhs_dmd_barcodes SET display='Aspirin tablets',vmp_name='Aspirin tablets',amp_code='AMP1'; INSERT INTO nhs_dmd_barcodes(gtin,code,display,system,amp_code,created_at,updated_at) VALUES('5016298210989','222','Warfarin tablets','https://dmd.nhs.uk','AMP2',now(),now()); INSERT INTO nhs_dmd_trade_family_groups(id,code,name,created_at,updated_at) VALUES(92001,'GROUP1','Synthetic group',now(),now()); INSERT INTO nhs_dmd_trade_families(id,code,name,trade_family_group_id,created_at,updated_at) VALUES(92001,'FAMILY1','Synthetic family',92001,now(),now()); INSERT INTO nhs_dmd_amp_trade_families(amp_code,trade_family_id,created_at,updated_at) VALUES('AMP1',92001,now(),now()),('AMP2',92001,now(),now()); INSERT INTO medication_review_evidence_records(id,source_name,source_record_id,source_url,product_name,label_section,evidence_text,retrieved_on,risk_level,match_confidence,match_status,candidate_terms,interacting_terms,created_at,updated_at) VALUES(90002,'Synthetic reference','lookup-1','https://example.test/evidence/lookup','Aspirin','Warnings','Avoid warfarin with aspirin.',date '2026-01-01','high','high','reviewed_pair','{aspirin}','{warfarin}',now(),now())").await.unwrap();
    let token = app.token().await;
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/medication_lookup?q=5000168511017",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body = response.json::<Value>().await.unwrap();
    app.close().await;
    assert_eq!(status, 200, "{body}");
    let result = &body["results"][0];
    assert_eq!(result["trade_family"]["code"], "FAMILY1", "{body}");
    assert_eq!(result["trade_family_group"]["code"], "GROUP1", "{body}");
    assert_eq!(result["related_medications"][0]["id"], 80001, "{body}");
    assert_eq!(
        result["related_medications"][0]["path"],
        "/households/persistence-fixture/medications/80001",
        "{body}"
    );
    assert!(
        result["related_medications"][0]
            .get("refill_path")
            .is_none(),
        "{body}"
    );
    assert_eq!(
        result["review_prompts"][0]["evidence_record_id"], 90002,
        "{body}"
    );
    assert_eq!(
        result["review_prompts"][0]["interacting_medication_name"],
        "Warfarin"
    );
    assert!(result.get("existing_medication").is_none());
}

#[tokio::test]
async fn medication_lookup_keeps_catalogue_results_when_review_evidence_is_unavailable() {
    let app = Application::new().await;
    seed_catalog(&app).await;
    app.fixture.admin.execute_unprepared("ALTER TABLE medication_review_evidence_records RENAME TO unavailable_review_evidence_records").await.unwrap();
    let token = app.token().await;
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/medication_lookup?q=5000168511017",
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
    assert_eq!(body["results"][0]["code"], "123456", "{body}");
    assert_eq!(body["review_guidance"]["status"], "unavailable", "{body}");
}

#[tokio::test]
async fn medication_lookup_matches_pack_names_without_confusing_strengths() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("UPDATE medications SET name='Metformin',dose_amount=500,dose_unit='mg',barcode=NULL,dmd_code=NULL WHERE id=80001; INSERT INTO medications(id,household_id,location_id,name,dose_amount,dose_unit,created_at,updated_at) VALUES(80003,72001,79001,'Metformin',1000,'mg',now(),now()); INSERT INTO nhs_dmd_barcodes(gtin,code,display,system,created_at,updated_at) VALUES('5000168511017','NEW','Metformin 500mg 28 tablets (Synthetic manufacturer)','https://dmd.nhs.uk',now(),now())").await.unwrap();
    let token = app.token().await;
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/medication_lookup?q=5000168511017",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body = response.json::<Value>().await.unwrap();
    app.close().await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body["results"][0]["existing_medication"]["id"], 80001,
        "{body}"
    );
    assert_eq!(
        body["results"][0]["existing_medication"]["path"],
        "/households/persistence-fixture/medications/80001",
        "{body}"
    );
    assert_eq!(
        body["results"][0]["existing_medication"]["refill_path"],
        "/households/persistence-fixture/medications/80001?refill=true",
        "{body}"
    );
    assert_eq!(
        body["query"], "Metformin 500mg 28 tablets (Synthetic manufacturer)",
        "{body}"
    );
}

#[tokio::test]
async fn medication_lookup_searches_names_case_insensitively_and_accepts_form_aliases() {
    let app = Application::new().await;
    seed_catalog(&app).await;
    let token = app.token().await;
    let mut observed = Vec::new();
    for query in ["synthetic", "SYNTHETIC", "SyNtHeTiC"] {
        let response = app
            .client
            .get(format!(
                "{}/api/v1/households/72001/medication_lookup?q={query}&form=caplet",
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
    app.close().await;
    for (status, body) in observed {
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["results"][0]["code"], "123456", "{body}");
        assert!(body["results"][0]["match_reason"].is_null());
        assert_eq!(body["form"], "tablet", "{body}");
    }
}

#[tokio::test]
async fn medication_lookup_normalizes_strength_and_preserves_distinct_gtins() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("INSERT INTO nhs_dmd_barcodes(gtin,code,display,system,created_at,updated_at) VALUES('5000168511017','STRENGTH','Synthetic 1mg tablets','https://dmd.nhs.uk',now(),now()),('5000168511018','STRENGTH','Synthetic 1mg tablets','https://dmd.nhs.uk',now(),now())").await.unwrap();
    let token = app.token().await;
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/medication_lookup?q=Synthetic&strength=1000mcg&form=caplet",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body = response.json::<Value>().await.unwrap();
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

async fn seed_catalog(app: &Application) {
    app.fixture.admin.execute_unprepared("INSERT INTO nhs_dmd_barcodes(gtin,code,display,vmp_name,system,concept_class,created_at,updated_at) VALUES('05000168511017','123456','Synthetic tablets 28 tablets','Synthetic tablets','https://dmd.nhs.uk','AMPP',now(),now()); UPDATE medications SET dmd_code='123456',dmd_system='https://dmd.nhs.uk' WHERE id=80001").await.unwrap();
}

#[tokio::test]
async fn medication_lookup_resolves_imported_gtin_and_unique_visible_stock() {
    let app = Application::new().await;
    seed_catalog(&app).await;
    let token = app.token().await;
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/medication_lookup?q=5000168511017",
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
    assert_eq!(
        body["barcode_resolution"],
        json!({"status":"resolved","source":"nhs_dmd"})
    );
    assert_eq!(body["results"][0]["code"], "123456");
    assert_eq!(body["results"][0]["existing_medication"]["id"], 80001);
    assert_eq!(body["permissions"]["can_create"], true);
}

#[tokio::test]
async fn medication_lookup_never_picks_an_ambiguous_stock_record() {
    let app = Application::new().await;
    seed_catalog(&app).await;
    app.fixture.admin.execute_unprepared("INSERT INTO medications(id,household_id,location_id,name,dmd_code,dmd_system,current_supply,created_at,updated_at) VALUES(80003,72001,79001,'Second cabinet tablets','123456','https://dmd.nhs.uk',4,now(),now())").await.unwrap();
    let token = app.token().await;
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/medication_lookup?q=5000168511017",
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
    assert_eq!(body["results"].as_array().unwrap().len(), 1);
    assert!(body["results"][0].get("existing_medication").is_none());
}

#[tokio::test]
async fn medication_lookup_rejects_unauthenticated_and_foreign_households() {
    let app = Application::new().await;
    let token = app.token().await;
    let anonymous = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/medication_lookup?q=5000168511017",
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
            "{}/api/v1/households/72002/medication_lookup?q=5000168511017",
            app.origin
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
async fn medication_lookup_preserves_local_source_priority_and_manual_misses() {
    let app = Application::new().await;
    seed_catalog(&app).await;
    app.fixture.admin.execute_unprepared("INSERT INTO barcode_catalog_entries(gtin,display,source,code,system,created_at,updated_at) VALUES('5000168511017','Imported custom medicine','household_import','CUSTOM','Local catalogue',now(),now())").await.unwrap();
    let token = app.token().await;
    let url = format!("{}/api/v1/households/72001/medication_lookup", app.origin);
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
    let token = app.token().await;
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/medication_lookup?q=5057753926137",
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
