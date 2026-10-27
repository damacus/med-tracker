use super::*;
use med_tracker::models::{
    care::{dosages, stock_check},
    entities::{dosage, version},
};
use sea_orm::{ActiveModelTrait, EntityTrait, PaginatorTrait, Set};

#[tokio::test]
async fn stock_check_updates_scalar_and_option_supply_with_aggregate_audit() {
    let fixture = Fixture::new().await;
    let now = chrono::Utc::now().naive_utc();
    let scalar = medication::ActiveModel {
        household_id: Set(fixture.household.id),
        created_by_membership_id: Set(Some(fixture.membership.id)),
        location_id: Set(fixture.location_id),
        name: Set(Some("Synthetic liquid".into())),
        current_supply: Set(Some("8".parse().unwrap())),
        dose_amount: Set(Some(1.0)),
        dose_unit: Set(Some("ml".into())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    let tenant = access::begin(&fixture.runtime, &fixture.scope, None)
        .await
        .unwrap();
    let option = dosages::create(
        &tenant,
        dosages::DosageInput::from_form(
            dosages::FormDosageInput {
                medication_id: Some(fixture.medication_id.to_string()),
                amount: "2".into(),
                unit: "tablet".into(),
                frequency: "daily".into(),
                description: None,
                default_for_adults: None,
                default_for_children: None,
                max_daily_doses: "3".into(),
                min_hours_between_doses: "4".into(),
                dose_cycle: "daily".into(),
                current_supply: Some(Some("4".into())),
                reorder_threshold: Some(Some("1".into())),
            },
            true,
        ),
        None,
    )
    .await
    .unwrap();
    tenant.commit().await.unwrap();

    let tenant = access::begin(&fixture.runtime, &fixture.scope, None)
        .await
        .unwrap();
    let rows = stock_check::read(&tenant, None).await.unwrap().rows;
    let submitted = [
        (format!("option_{}", option.id), "2.25"),
        (format!("medication_{}", scalar.id), "5.75"),
    ]
    .into_iter()
    .map(|(key, quantity)| {
        let row = rows.iter().find(|row| row.key == key).unwrap();
        stock_check::SubmittedRow {
            key,
            snapshot: row.snapshot.clone(),
            quantity: quantity.into(),
        }
    })
    .collect();
    let prior_audits = version::Entity::find()
        .filter(version::Column::HouseholdId.eq(fixture.household.id))
        .count(&fixture.admin)
        .await
        .unwrap();
    let prior_changes = fixture.changes().count(&fixture.admin).await.unwrap();
    stock_check::apply(&tenant, submitted, "house_stock_check", None)
        .await
        .unwrap();
    tenant.commit().await.unwrap();

    let option = dosage::Entity::find_by_id(option.id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let aggregate = medication::Entity::find_by_id(fixture.medication_id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let scalar = medication::Entity::find_by_id(scalar.id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let audits = version::Entity::find()
        .filter(version::Column::HouseholdId.eq(fixture.household.id))
        .count(&fixture.admin)
        .await
        .unwrap();
    let changes = fixture.changes().count(&fixture.admin).await.unwrap();
    assert_eq!(
        option.current_supply.unwrap().normalize().to_string(),
        "2.25"
    );
    assert_eq!(
        aggregate.current_supply.unwrap().normalize().to_string(),
        "2.25"
    );
    assert_eq!(
        scalar.current_supply.unwrap().normalize().to_string(),
        "5.75"
    );
    assert_eq!(audits - prior_audits, 3);
    assert_eq!(changes - prior_changes, 3);
    fixture.close().await;
}
