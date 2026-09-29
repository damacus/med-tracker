use medtracker_contract_tests::{fixture, Target};
use serde_json::Value;

#[test]
fn stocked_medication_forecast_uses_active_daily_schedule_on_list_and_show() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = format!("/api/v1/households/{}/medications", fixture.household_id);
    let list = target.get(&path, Some(&fixture.access_token));
    assert_eq!(list.status().as_u16(), 200);
    let body: Value = list.json().unwrap();
    let medication = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|medication| medication["id"] == fixture.managed_medication_id)
        .unwrap();
    assert_eq!(medication["days_until_low_stock"], 12);
    assert_eq!(medication["days_until_out_of_stock"], 13);

    let show = target.get(
        &format!("{path}/{}", fixture.managed_medication_id),
        Some(&fixture.access_token),
    );
    assert_eq!(show.status().as_u16(), 200);
    let body: Value = show.json().unwrap();
    assert_eq!(body["data"]["days_until_low_stock"], 12);
    assert_eq!(body["data"]["days_until_out_of_stock"], 13);
}

#[test]
fn untracked_medication_keeps_forecasts_null() {
    let fixture = fixture();
    let target = Target::from_env();
    let show = target.get(
        &format!(
            "/api/v1/households/{}/medications/{}",
            fixture.medication_read_household_id, fixture.medication_read_unlinked_id
        ),
        Some(&fixture.medication_read_delegated_access_token),
    );
    assert_eq!(show.status().as_u16(), 200);
    let body: Value = show.json().unwrap();
    assert!(body["data"]["days_until_low_stock"].is_null());
    assert!(body["data"]["days_until_out_of_stock"].is_null());
}

#[test]
fn weekly_schedule_and_assignment_rates_are_added_before_rounding() {
    let fixture = fixture();
    let target = Target::from_env();
    let show = target.get(
        &format!(
            "/api/v1/households/{}/medications/{}",
            fixture.household_id, fixture.forecast_medication_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(show.status().as_u16(), 200);
    let body: Value = show.json().unwrap();
    assert_eq!(body["data"]["days_until_low_stock"], 1);
    assert_eq!(body["data"]["days_until_out_of_stock"], 2);
}
