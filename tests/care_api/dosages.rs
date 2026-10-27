use super::*;
use sea_orm::sea_query::{Expr as QueryExpr, ExprTrait as _, Func as QueryFunc, Query};

fn dosage_body(app: &Application) -> Value {
    let fixture_medication_id = app.fixture.medication.id;
    json!({"dosage_option":{"medication_id":fixture_medication_id.to_string(),"amount":"2","unit":"tablet","frequency":"daily","default_max_daily_doses":3,"default_min_hours_between_doses":"4","default_dose_cycle":"daily","current_supply":"12","reorder_threshold":"3"}})
}

#[tokio::test]
async fn dosage_conformance_tracked_description_update_retains_parent_inventory_callback() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/dosage_options",
        app.origin
    );
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&dosage_body(&app))
        .send()
        .await
        .unwrap();
    let body: Value = created.json().await.unwrap();
    let id = body["data"]["id"].as_i64().unwrap();
    med_tracker::models::_entities::medications::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::medications::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(
                "2020-01-01T00:00:00"
                    .parse::<chrono::NaiveDateTime>()
                    .unwrap(),
            ),
        )
        .filter(med_tracker::models::_entities::medications::Column::Id.eq(fixture_medication_id))
        .filter(
            med_tracker::models::_entities::medications::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let response = app
        .client
        .patch(format!("{endpoint}/{id}"))
        .bearer_auth(&token)
        .json(&json!({"dosage_option":{"description":"Synthetic description only"}}))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(QueryExpr::col("current_supply").cast_as("text"), "supply")
                .expr_as(
                    QueryExpr::col("reorder_threshold").cast_as("text"),
                    "threshold",
                )
                .expr_as(
                    QueryExpr::col("updated_at").gt(QueryExpr::val(
                        "2020-01-01T00:00:00"
                            .parse::<chrono::NaiveDateTime>()
                            .unwrap(),
                    )),
                    "refreshed",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("versions")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("item_type")
                                            .eq(QueryExpr::val("Medication")),
                                    )
                                    .and(QueryExpr::col("event").eq(QueryExpr::val("api_update"))),
                            )
                            .to_owned(),
                    ),
                    "versions",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("versions")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("item_type")
                                            .eq(QueryExpr::val("MedicationDosageOption")),
                                    )
                                    .and(QueryExpr::col("item_id").eq(QueryExpr::val(id)))
                                    .and(QueryExpr::col("event").eq(QueryExpr::val("api_update"))),
                            )
                            .to_owned(),
                    ),
                    "option_updates",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("api_change_events")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("record_type")
                                            .eq(QueryExpr::val("Medication")),
                                    )
                                    .and(QueryExpr::col("action").eq(QueryExpr::val("update"))),
                            )
                            .to_owned(),
                    ),
                    "changes",
                )
                .from("medications")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("id").eq(QueryExpr::val(fixture_medication_id))),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let supply: String = row.try_get("", "supply").unwrap();
    let threshold: String = row.try_get("", "threshold").unwrap();
    let refreshed: bool = row.try_get("", "refreshed").unwrap();
    let versions: i64 = row.try_get("", "versions").unwrap();
    let option_updates: i64 = row.try_get("", "option_updates").unwrap();
    let changes: i64 = row.try_get("", "changes").unwrap();
    app.close().await;
    assert_eq!(status, 200);
    assert_eq!((supply.as_str(), threshold.as_str()), ("12.00", "3.00"));
    assert!(refreshed);
    assert_eq!((versions, changes), (2, 2));
    assert_eq!(option_updates, 1);
}

#[tokio::test]
async fn dosage_conformance_untracked_creation_retains_raw_parent_mode_sync_without_version() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
    let mut body = dosage_body(&app);
    body["dosage_option"]["current_supply"] = Value::Null;
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/{fixture_household_id}/dosage_options",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(QueryExpr::col("dose_amount").is_null(), "mode")
                .expr_as(QueryExpr::col("current_supply").cast_as("text"), "supply")
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("versions")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("item_type")
                                            .eq(QueryExpr::val("Medication")),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "parent_versions",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("versions")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("item_type")
                                            .eq(QueryExpr::val("MedicationDosageOption")),
                                    )
                                    .and(QueryExpr::col("event").eq(QueryExpr::val("api_create"))),
                            )
                            .to_owned(),
                    ),
                    "option_versions",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("api_change_events")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("record_type")
                                            .eq(QueryExpr::val("Medication")),
                                    )
                                    .and(QueryExpr::col("action").eq(QueryExpr::val("update"))),
                            )
                            .to_owned(),
                    ),
                    "changes",
                )
                .from("medications")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("id").eq(QueryExpr::val(fixture_medication_id))),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let mode: bool = row.try_get("", "mode").unwrap();
    let supply: String = row.try_get("", "supply").unwrap();
    let counts: Vec<i64> = ["parent_versions", "option_versions", "changes"]
        .iter()
        .map(|field| row.try_get("", field).unwrap())
        .collect();
    app.close().await;
    assert_eq!(status, 201);
    assert!(mode);
    assert_eq!(supply, "10.00");
    assert_eq!(counts, vec![0, 1, 1]);
}

async fn seed_removable_options(
    app: &Application,
) -> (
    med_tracker::models::_entities::dosages::Model,
    med_tracker::models::_entities::dosages::Model,
) {
    let fixture_household_id = app.fixture.household.id;
    let fixture_medication_id = app.fixture.medication.id;
    let _dosages_82001 = med_tracker::models::_entities::dosages::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        medication_id: sea_orm::Set(fixture_medication_id),
        amount: sea_orm::Set("2".parse::<sea_orm::prelude::Decimal>().unwrap()),
        unit: sea_orm::Set("tablet".to_owned()),
        frequency: sea_orm::Set("daily".to_owned()),
        current_supply: sea_orm::Set(Some("6".parse::<sea_orm::prelude::Decimal>().unwrap())),
        reorder_threshold: sea_orm::Set(Some("3".parse::<sea_orm::prelude::Decimal>().unwrap())),
        default_dose_cycle: sea_orm::Set(0),
        default_max_daily_doses: sea_orm::Set(4),
        default_min_hours_between_doses: sea_orm::Set(
            "0".parse::<sea_orm::prelude::Decimal>().unwrap(),
        ),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _dosages_82002 = med_tracker::models::_entities::dosages::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        medication_id: sea_orm::Set(fixture_medication_id),
        amount: sea_orm::Set("1".parse::<sea_orm::prelude::Decimal>().unwrap()),
        unit: sea_orm::Set("tablet".to_owned()),
        frequency: sea_orm::Set("daily".to_owned()),
        current_supply: sea_orm::Set(Some("4".parse::<sea_orm::prelude::Decimal>().unwrap())),
        reorder_threshold: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
        default_dose_cycle: sea_orm::Set(0),
        default_max_daily_doses: sea_orm::Set(4),
        default_min_hours_between_doses: sea_orm::Set(
            "0".parse::<sea_orm::prelude::Decimal>().unwrap(),
        ),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    med_tracker::models::_entities::medications::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::medications::Column::DoseAmount,
            sea_orm::sea_query::Expr::value(None::<f64>),
        )
        .col_expr(
            med_tracker::models::_entities::medications::Column::CurrentSupply,
            sea_orm::sea_query::Expr::value(Some(
                "10".parse::<sea_orm::prelude::Decimal>().unwrap(),
            )),
        )
        .col_expr(
            med_tracker::models::_entities::medications::Column::ReorderThreshold,
            sea_orm::sea_query::Expr::value("5".parse::<sea_orm::prelude::Decimal>().unwrap()),
        )
        .col_expr(
            med_tracker::models::_entities::medications::Column::SupplyAtLastRestock,
            sea_orm::sea_query::Expr::value(Some(
                "10".parse::<sea_orm::prelude::Decimal>().unwrap(),
            )),
        )
        .filter(med_tracker::models::_entities::medications::Column::Id.eq(fixture_medication_id))
        .filter(
            med_tracker::models::_entities::medications::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();

    (_dosages_82001, _dosages_82002)
}

async fn removal_context(
    app: &Application,
) -> (
    med_tracker::models::identity::resource::ValidatedPrincipal,
    med_tracker::models::access::TenantTransaction,
) {
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("authorization", format!("Bearer {token}").parse().unwrap());
    let principal =
        med_tracker::models::identity::resource::authenticate(&app.fixture.runtime, &headers, None)
            .await
            .unwrap();
    let tenant = principal
        .begin_household(
            &app.fixture.runtime,
            fixture_household_id,
            "synthetic-remove-dosage".to_owned(),
            None,
        )
        .await
        .unwrap();
    (principal, tenant)
}

#[tokio::test]
async fn dosage_removal_recalculates_parent_and_records_atomic_history() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_medication_id = app.fixture.medication.id;
    let (_dosages_82001, _dosages_82002) = seed_removable_options(&app).await;
    let (principal, tenant) = removal_context(&app).await;
    let result = med_tracker::models::care::dosages::destroy(
        &tenant,
        &_dosages_82001.id.to_string(),
        Some(principal.provenance()),
    )
    .await;
    if result.is_ok() {
        tenant.commit().await.unwrap();
    } else {
        tenant.rollback().await.unwrap();
    }
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(QueryExpr::col("current_supply").cast_as("text"), "supply")
                .expr_as(
                    QueryExpr::col("reorder_threshold").cast_as("text"),
                    "threshold",
                )
                .expr_as(
                    QueryExpr::col("supply_at_last_restock").cast_as("text"),
                    "baseline",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("dosages")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("id").eq(QueryExpr::val(_dosages_82001.id)),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "removed",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("versions")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("item_type")
                                            .eq(QueryExpr::val("MedicationDosageOption")),
                                    )
                                    .and(QueryExpr::col("event").eq(QueryExpr::val("destroy"))),
                            )
                            .to_owned(),
                    ),
                    "versions",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("api_tombstones")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("record_type")
                                            .eq(QueryExpr::val("MedicationDosageOption")),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "tombstones",
                )
                .from("medications")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("id").eq(QueryExpr::val(fixture_medication_id))),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let values: Vec<String> = ["supply", "threshold", "baseline"]
        .iter()
        .map(|field| row.try_get("", field).unwrap())
        .collect();
    let counts: Vec<i64> = ["removed", "versions", "tombstones"]
        .iter()
        .map(|field| row.try_get("", field).unwrap())
        .collect();
    app.close().await;
    assert!(result.is_ok());
    assert_eq!(values, vec!["4.00", "2.00", "10.00"]);
    assert_eq!(counts, vec![0, 1, 1]);
}

#[tokio::test]
async fn dosage_removal_protects_inactive_schedule_and_assignment_references() {
    for reference in ["assignment", "schedule"] {
        let app = Application::new().await;
        let fixture_household_id = app.fixture.household.id;
        let fixture_person_id = app.fixture.account.person_id;
        let fixture_medication_id = app.fixture.medication.id;
        let fixture_assignment_id = app.fixture.assignment.id;
        let (_dosages_82001, _dosages_82002) = seed_removable_options(&app).await;
        if reference == "assignment" {
            med_tracker::models::_entities::person_medications::Entity::update_many().col_expr(med_tracker::models::_entities::person_medications::Column::SourceDosageOptionId, sea_orm::sea_query::Expr::value(Some(_dosages_82001.id))).col_expr(med_tracker::models::_entities::person_medications::Column::Active, sea_orm::sea_query::Expr::value(false)).filter(med_tracker::models::_entities::person_medications::Column::Id.eq(fixture_assignment_id)).filter(med_tracker::models::_entities::person_medications::Column::HouseholdId.eq(app.fixture.household.id)).exec(&app.fixture.admin).await.unwrap();
        } else {
            let _schedules_83001 = med_tracker::models::_entities::schedules::ActiveModel {
                household_id: sea_orm::Set(fixture_household_id),
                person_id: sea_orm::Set(fixture_person_id),
                medication_id: sea_orm::Set(fixture_medication_id),
                source_dosage_option_id: sea_orm::Set(Some(_dosages_82001.id)),
                active: sea_orm::Set(false),
                start_date: sea_orm::Set(Some("2026-10-05".parse::<chrono::NaiveDate>().unwrap())),
                end_date: sea_orm::Set(Some("2026-10-10".parse::<chrono::NaiveDate>().unwrap())),
                dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
                dose_unit: sea_orm::Set(Some("tablet".to_owned())),
                schedule_type: sea_orm::Set(6),
                schedule_config: sea_orm::Set(
                    serde_json::from_str::<serde_json::Value>("{}").unwrap(),
                ),
                created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
                updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
                ..Default::default()
            }
            .insert(&app.fixture.admin)
            .await
            .unwrap();
        }
        let (principal, tenant) = removal_context(&app).await;
        let result = med_tracker::models::care::dosages::destroy(
            &tenant,
            &_dosages_82001.id.to_string(),
            Some(principal.provenance()),
        )
        .await;
        tenant.rollback().await.unwrap();
        let row = app
            .fixture
            .admin
            .query_one_raw({
                let query = Query::select()
                    .expr_as(
                        QueryExpr::expr(
                            Query::select()
                                .expr(QueryExpr::expr(QueryFunc::count(
                                    sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                                )))
                                .from("dosages")
                                .and_where(
                                    QueryExpr::col("household_id")
                                        .eq(QueryExpr::val(fixture_household_id))
                                        .and(
                                            QueryExpr::col("id")
                                                .eq(QueryExpr::val(_dosages_82001.id)),
                                        ),
                                )
                                .to_owned(),
                        ),
                        "options",
                    )
                    .expr_as(
                        QueryExpr::expr(
                            Query::select()
                                .expr(QueryExpr::expr(QueryFunc::count(
                                    sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                                )))
                                .from("versions")
                                .and_where(
                                    QueryExpr::col("household_id")
                                        .eq(QueryExpr::val(fixture_household_id))
                                        .and(
                                            QueryExpr::col("item_type")
                                                .eq(QueryExpr::val("MedicationDosageOption")),
                                        ),
                                )
                                .to_owned(),
                        ),
                        "versions",
                    )
                    .to_owned();
                app.fixture.admin.get_database_backend().build(&query)
            })
            .await
            .unwrap()
            .unwrap();
        let counts: Vec<i64> = ["options", "versions"]
            .iter()
            .map(|field| row.try_get("", field).unwrap())
            .collect();
        app.close().await;
        assert!(
            matches!(
                result,
                Err(med_tracker::models::errors::OperationError::Validation { .. })
            ),
            "{reference}"
        );
        assert_eq!(counts, vec![1, 0]);
    }
}

#[tokio::test]
// Whole-run exclusive: PostgreSQL trigger functions, trigger installation and sequence privileges are unsupported by SeaORM Schema and SeaQuery Table/Query builders. The raw DDL is limited to this case and removed before closing.
async fn dosage_removal_audit_rejection_rolls_back_deleted_option_and_parent_stock() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_medication_id = app.fixture.medication.id;
    let (_dosages_82001, _dosages_82002) = seed_removable_options(&app).await;
    app.fixture
        .admin
        .execute_unprepared("CREATE SEQUENCE dosage_removal_audit_reached")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared(
            "GRANT USAGE,SELECT ON SEQUENCE dosage_removal_audit_reached TO med_tracker_app",
        )
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared(&format!("CREATE FUNCTION reject_dosage_removal_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='MedicationDosageOption' AND NEW.event='destroy' THEN IF NOT EXISTS(SELECT 1 FROM dosages WHERE id={fixture_82001}) AND (SELECT current_supply FROM medications WHERE id={fixture_medication_id})=4 THEN PERFORM nextval('dosage_removal_audit_reached'); END IF; RAISE EXCEPTION 'Synthetic removal audit rejection'; END IF; RETURN NEW; END $$", fixture_82001 = _dosages_82001.id)).await.unwrap();
    app.fixture.admin.execute_unprepared("CREATE TRIGGER reject_dosage_removal_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_dosage_removal_audit()").await.unwrap();
    let (principal, tenant) = removal_context(&app).await;
    let result = med_tracker::models::care::dosages::destroy(
        &tenant,
        &_dosages_82001.id.to_string(),
        Some(principal.provenance()),
    )
    .await;
    tenant.rollback().await.unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::col("is_called"))
                            .from("dosage_removal_audit_reached")
                            .to_owned(),
                    ),
                    "reached",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("dosages")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("id").eq(QueryExpr::val(_dosages_82001.id)),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "options",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::col("current_supply").cast_as("text"))
                            .from("medications")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("id")
                                            .eq(QueryExpr::val(fixture_medication_id)),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "supply",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("api_tombstones")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("record_type")
                                            .eq(QueryExpr::val("MedicationDosageOption")),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "tombstones",
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let reached: bool = row.try_get("", "reached").unwrap();
    let options: i64 = row.try_get("", "options").unwrap();
    let supply: String = row.try_get("", "supply").unwrap();
    let tombstones: i64 = row.try_get("", "tombstones").unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP TRIGGER IF EXISTS reject_dosage_removal_audit ON versions")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP FUNCTION IF EXISTS reject_dosage_removal_audit()")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP SEQUENCE IF EXISTS dosage_removal_audit_reached")
        .await
        .unwrap();
    app.close().await;
    assert!(result.is_err());
    assert!(reached);
    assert_eq!((options, supply.as_str(), tombstones), (1, "10.00", 0));
}

#[tokio::test]
async fn dosage_removal_requires_current_manager_and_scoped_parent() {
    let app = Application::new().await;
    let fixture_membership_id = app.fixture.membership.id;
    let (_dosages_82001, _dosages_82002) = seed_removable_options(&app).await;
    let (principal, tenant) = removal_context(&app).await;
    med_tracker::models::_entities::household_memberships::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::household_memberships::Column::Role,
            sea_orm::sea_query::Expr::value("member".to_owned()),
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
    let denied = med_tracker::models::care::dosages::destroy(
        &tenant,
        &_dosages_82001.id.to_string(),
        Some(principal.provenance()),
    )
    .await;
    tenant.rollback().await.unwrap();
    app.close().await;
    assert!(matches!(
        denied,
        Err(med_tracker::models::errors::OperationError::Forbidden)
    ));
}

#[tokio::test]
async fn dosage_api_create_is_immediately_readable_and_inventory_audit_are_atomic() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/dosage_options",
        app.origin
    );
    let response = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&dosage_body(&app))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let etag = response
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let body: Value = response.json().await.unwrap_or(Value::Null);
    let id = body["data"]["id"].as_i64().unwrap_or(0);
    let read = app
        .client
        .get(format!("{endpoint}/{id}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let read_status = read.status().as_u16();
    let collection = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let collection_status = collection.status().as_u16();
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(QueryExpr::col("dose_amount").is_null(), "options_mode")
                .expr_as(QueryExpr::col("current_supply").cast_as("text"), "supply")
                .expr_as(
                    QueryExpr::col("reorder_threshold").cast_as("text"),
                    "threshold",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("versions")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("item_type")
                                            .eq(QueryExpr::val("MedicationDosageOption")),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "versions",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("api_change_events")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("record_type")
                                            .eq(QueryExpr::val("MedicationDosageOption")),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "changes",
                )
                .from("medications")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(QueryExpr::col("id").eq(QueryExpr::val(fixture_medication_id))),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let mode: bool = row.try_get("", "options_mode").unwrap();
    let supply: String = row.try_get("", "supply").unwrap();
    let threshold: String = row.try_get("", "threshold").unwrap();
    let versions: i64 = row.try_get("", "versions").unwrap();
    let changes: i64 = row.try_get("", "changes").unwrap();
    app.close().await;
    assert_eq!((status, read_status, collection_status), (201, 200, 200));
    assert!(!etag.is_empty());
    assert_eq!(body["data"]["amount"], "2.0");
    assert!(mode);
    assert_eq!((supply.as_str(), threshold.as_str()), ("12.00", "3.00"));
    assert_eq!((versions, changes), (1, 1));
}

#[tokio::test]
async fn dosage_api_partial_update_and_optional_stale_etag_preserve_existing_values() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/dosage_options",
        app.origin
    );
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&dosage_body(&app))
        .send()
        .await
        .unwrap();
    let etag = created
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let body: Value = created.json().await.unwrap_or(Value::Null);
    let id = body["data"]["id"].as_i64().unwrap_or(0);
    let resource = format!("{endpoint}/{id}");
    let updated = app
        .client
        .patch(&resource)
        .bearer_auth(&token)
        .json(&json!({"dosage_option":{"description":"Synthetic revised option"}}))
        .send()
        .await
        .unwrap();
    let updated_status = updated.status().as_u16();
    let updated: Value = updated.json().await.unwrap_or(Value::Null);
    let stale = app
        .client
        .put(&resource)
        .bearer_auth(&token)
        .header("if-match", etag)
        .json(&json!({"dosage_option":{"amount":"4"}}))
        .send()
        .await
        .unwrap();
    let stale_status = stale.status().as_u16();
    app.close().await;
    assert_eq!((updated_status, stale_status), (200, 409));
    assert_eq!(updated["data"]["amount"], "2.0");
    assert_eq!(updated["data"]["description"], "Synthetic revised option");
}

#[tokio::test]
async fn dosage_api_invalid_and_duplicate_defaults_have_no_partial_effect() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{fixture_household_id}/dosage_options",
        app.origin
    );
    let mut invalid = dosage_body(&app);
    invalid["dosage_option"]["amount"] = json!("0");
    let rejected = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&invalid)
        .send()
        .await
        .unwrap();
    let mut valid = dosage_body(&app);
    valid["dosage_option"]["default_for_adults"] = json!(true);
    let first = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&valid)
        .send()
        .await
        .unwrap();
    let duplicate = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&valid)
        .send()
        .await
        .unwrap();
    let statuses = (
        rejected.status().as_u16(),
        first.status().as_u16(),
        duplicate.status().as_u16(),
    );
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(
                    QueryExpr::expr(QueryFunc::count(sea_orm::sea_query::Expr::col(
                        sea_orm::sea_query::Asterisk,
                    ))),
                    "options",
                )
                .from("dosages")
                .and_where(
                    QueryExpr::col("household_id")
                        .eq(QueryExpr::val(fixture_household_id))
                        .and(
                            QueryExpr::col("medication_id")
                                .eq(QueryExpr::val(fixture_medication_id)),
                        ),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let options: i64 = row.try_get("", "options").unwrap();
    app.close().await;
    assert_eq!(statuses, (422, 201, 422));
    assert_eq!(options, 1);
}

#[tokio::test]
async fn dosage_api_member_write_denial_precedes_validation_and_foreign_reads_are_hidden() {
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_membership_id = app.fixture.membership.id;
    let foreign_household = app.fixture.seed_foreign_household(0).await;
    let token = app.token().await;
    med_tracker::models::_entities::household_memberships::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::household_memberships::Column::Role,
            sea_orm::sea_query::Expr::value("member".to_owned()),
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
    let denied = app
        .client
        .post(format!(
            "{}/api/v1/households/{fixture_household_id}/dosage_options",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_72002}/dosage_options",
            app.origin,
            fixture_72002 = foreign_household.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let statuses = (denied.status().as_u16(), foreign.status().as_u16());
    app.close().await;
    assert_eq!(statuses, (403, 403));
}

#[tokio::test]
async fn dosage_option_get_matches_documented_contract() {
    use super::contract::{assert_value, contract, resolve};
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_location_id = app.fixture.location.id;
    let fixture_medication_id = app.fixture.medication.id;
    let foreign_household = app.fixture.seed_foreign_household(0).await;
    let token = app.token().await;
    let collection = format!(
        "{}/api/v1/households/{fixture_household_id}/dosage_options",
        app.origin
    );
    let contract = contract();
    let operation = &contract["paths"]["/households/{household_id}/dosage_options/{id}"]["get"];
    assert_eq!(operation["operationId"], "getDosageOption");
    assert_eq!(
        operation["responses"]["404"]["$ref"],
        "#/components/responses/NotFound"
    );
    assert_eq!(
        operation["responses"]["403"]["$ref"],
        "#/components/responses/Forbidden"
    );
    assert!(
        operation["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .any(|parameter| parameter["$ref"] == "#/components/parameters/if_none_match"),
        "getDosageOption must document If-None-Match"
    );
    assert_eq!(
        operation["responses"]["304"]["headers"]["ETag"]["$ref"],
        "#/components/headers/etag"
    );
    let not_found = resolve(contract, &operation["responses"]["404"]);
    let not_found_schema = resolve(
        contract,
        &not_found["content"]["application/json"]["schema"],
    );
    let forbidden = resolve(contract, &operation["responses"]["403"]);
    let forbidden_schema = resolve(
        contract,
        &forbidden["content"]["application/json"]["schema"],
    );

    let created = app
        .client
        .post(&collection)
        .bearer_auth(&token)
        .json(&dosage_body(&app))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let created_body: Value = created.json().await.unwrap();
    let id = created_body["data"]["id"].as_i64().unwrap();
    let endpoint = format!("{collection}/{id}");

    let read = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let read_status = read.status().as_u16();
    let etag = read
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("missing")
        .to_owned();
    let body: Value = read.json().await.unwrap();
    assert_eq!(read_status, 200);
    assert_eq!(
        operation["responses"]["200"]["headers"]["ETag"]["$ref"],
        "#/components/headers/etag"
    );
    assert_ne!(etag, "missing");
    assert_eq!(
        operation["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/DosageOptionResponse"
    );
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "dosage option response",
    );
    assert_eq!(body["data"]["medication_id"], fixture_medication_id);
    assert_eq!(body["data"]["amount"], "2.0");

    let cached = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .header("if-none-match", &etag)
        .send()
        .await
        .unwrap();
    let cached_status = cached.status().as_u16();
    let cached_etag = cached
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("missing")
        .to_owned();
    let cached_body = cached.text().await.unwrap();
    assert_eq!(cached_status, 304);
    assert_eq!(cached_etag, etag);
    assert!(cached_body.is_empty());

    let missing = app
        .client
        .get(format!("{collection}/99999"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let missing_status = missing.status().as_u16();
    let missing_request_id = missing.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let missing_body: Value = missing.json().await.unwrap();
    assert_eq!(missing_status, 404);
    assert_value(
        contract,
        not_found_schema,
        &missing_body,
        "absent dosage option",
    );
    assert_eq!(missing_body["error"]["code"], "not_found");
    assert_eq!(missing_body["error"]["request_id"], missing_request_id);

    let _medications_92005 = med_tracker::models::_entities::medications::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        location_id: sea_orm::Set(fixture_location_id),
        name: sea_orm::Set(Some("Synthetic ungranted medicine".to_owned())),
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
    let _person_medications_92006 =
        med_tracker::models::_entities::person_medications::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            person_id: sea_orm::Set(fixture_minor_id),
            medication_id: sea_orm::Set(_medications_92005.id),
            dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
            dose_unit: sea_orm::Set(Some("tablet".to_owned())),
            position: sea_orm::Set(0),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let _dosages_92007 = med_tracker::models::_entities::dosages::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        medication_id: sea_orm::Set(_medications_92005.id),
        amount: sea_orm::Set("2".parse::<sea_orm::prelude::Decimal>().unwrap()),
        unit: sea_orm::Set("tablet".to_owned()),
        frequency: sea_orm::Set("daily".to_owned()),
        current_supply: sea_orm::Set(Some("6".parse::<sea_orm::prelude::Decimal>().unwrap())),
        reorder_threshold: sea_orm::Set(Some("3".parse::<sea_orm::prelude::Decimal>().unwrap())),
        default_dose_cycle: sea_orm::Set(0),
        default_max_daily_doses: sea_orm::Set(4),
        default_min_hours_between_doses: sea_orm::Set(
            "0".parse::<sea_orm::prelude::Decimal>().unwrap(),
        ),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    med_tracker::models::_entities::household_memberships::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::household_memberships::Column::Role,
            sea_orm::sea_query::Expr::value("member".to_owned()),
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
    let ungranted = app
        .client
        .get(format!(
            "{collection}/{fixture_92007}",
            fixture_92007 = _dosages_92007.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let ungranted_status = ungranted.status().as_u16();
    let ungranted_body: Value = ungranted.json().await.unwrap();
    assert_eq!(ungranted_status, 200);
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &ungranted_body,
        "participating person's dosage option",
    );
    assert_eq!(ungranted_body["data"]["id"], _dosages_92007.id);

    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_72002}/dosage_options/{id}",
            app.origin,
            fixture_72002 = foreign_household.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
    let foreign_request_id = foreign.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let foreign_body: Value = foreign.json().await.unwrap();
    assert_eq!(foreign_status, 403);
    assert_value(
        contract,
        forbidden_schema,
        &foreign_body,
        "foreign household",
    );
    assert_eq!(foreign_body["error"]["code"], "forbidden");
    assert_eq!(foreign_body["error"]["request_id"], foreign_request_id);
    app.close().await;
}

#[tokio::test]
async fn dosage_option_list_matches_documented_contract() {
    use super::contract::{assert_value, contract, resolve};
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_location_id = app.fixture.location.id;
    let foreign_household = app.fixture.seed_foreign_household(0).await;
    let token = app.token().await;
    let collection = format!(
        "{}/api/v1/households/{fixture_household_id}/dosage_options",
        app.origin
    );
    let contract = contract();
    let operation = &contract["paths"]["/households/{household_id}/dosage_options"]["get"];
    assert_eq!(operation["operationId"], "listDosageOptions");
    assert_eq!(
        operation["responses"]["422"]["$ref"],
        "#/components/responses/ValidationFailed"
    );
    assert_eq!(
        operation["responses"]["403"]["$ref"],
        "#/components/responses/Forbidden"
    );
    let invalid = resolve(contract, &operation["responses"]["422"]);
    let invalid_schema = resolve(contract, &invalid["content"]["application/json"]["schema"]);
    let forbidden = resolve(contract, &operation["responses"]["403"]);
    let forbidden_schema = resolve(
        contract,
        &forbidden["content"]["application/json"]["schema"],
    );

    let created = app
        .client
        .post(&collection)
        .bearer_auth(&token)
        .json(&dosage_body(&app))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let created_body: Value = created.json().await.unwrap();
    let id = created_body["data"]["id"].as_i64().unwrap();

    let list = app
        .client
        .get(&collection)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let list_status = list.status().as_u16();
    let body: Value = list.json().await.unwrap();
    assert_eq!(list_status, 200);
    assert_eq!(
        operation["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/DosageOptionCollectionResponse"
    );
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "dosage option collection",
    );
    assert_eq!(body["data"][0]["id"], id);
    assert_eq!(body["meta"]["total_count"], 1);

    for query in ["page=0", "per_page=101", "updated_since=invalid"] {
        let rejected = app
            .client
            .get(format!("{collection}?{query}"))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        let rejected_status = rejected.status().as_u16();
        let rejected_request_id = rejected.headers()["x-request-id"]
            .to_str()
            .unwrap()
            .to_owned();
        let rejected_body: Value = rejected.json().await.unwrap();
        assert_eq!(rejected_status, 422, "{query}");
        assert_value(contract, invalid_schema, &rejected_body, "invalid filter");
        assert_eq!(rejected_body["error"]["request_id"], rejected_request_id);
    }

    let _medications_92008 = med_tracker::models::_entities::medications::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        location_id: sea_orm::Set(fixture_location_id),
        name: sea_orm::Set(Some("Synthetic ungranted option medicine".to_owned())),
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
    let _person_medications_92009 =
        med_tracker::models::_entities::person_medications::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            person_id: sea_orm::Set(fixture_minor_id),
            medication_id: sea_orm::Set(_medications_92008.id),
            dose_amount: sea_orm::Set(Some("2".parse::<sea_orm::prelude::Decimal>().unwrap())),
            dose_unit: sea_orm::Set(Some("tablet".to_owned())),
            position: sea_orm::Set(0),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let _dosages_92012 = med_tracker::models::_entities::dosages::ActiveModel {
        household_id: sea_orm::Set(fixture_household_id),
        medication_id: sea_orm::Set(_medications_92008.id),
        amount: sea_orm::Set("2".parse::<sea_orm::prelude::Decimal>().unwrap()),
        unit: sea_orm::Set("tablet".to_owned()),
        frequency: sea_orm::Set("daily".to_owned()),
        current_supply: sea_orm::Set(Some("6".parse::<sea_orm::prelude::Decimal>().unwrap())),
        reorder_threshold: sea_orm::Set(Some("3".parse::<sea_orm::prelude::Decimal>().unwrap())),
        default_dose_cycle: sea_orm::Set(0),
        default_max_daily_doses: sea_orm::Set(4),
        default_min_hours_between_doses: sea_orm::Set(
            "0".parse::<sea_orm::prelude::Decimal>().unwrap(),
        ),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    med_tracker::models::_entities::household_memberships::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::household_memberships::Column::Role,
            sea_orm::sea_query::Expr::value("member".to_owned()),
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
    let scoped = app
        .client
        .get(&collection)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let scoped_status = scoped.status().as_u16();
    let scoped_body: Value = scoped.json().await.unwrap();
    assert_eq!(scoped_status, 200);
    assert_eq!(scoped_body["meta"]["total_count"], 2);
    assert_eq!(scoped_body["data"][0]["id"], id);
    assert_eq!(scoped_body["data"][1]["id"], _dosages_92012.id);

    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/{fixture_72002}/dosage_options",
            app.origin,
            fixture_72002 = foreign_household.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
    let foreign_body: Value = foreign.json().await.unwrap();
    assert_eq!(foreign_status, 403);
    assert_value(
        contract,
        forbidden_schema,
        &foreign_body,
        "foreign household dosage options",
    );
    assert_eq!(foreign_body["error"]["code"], "forbidden");
    app.close().await;
}
