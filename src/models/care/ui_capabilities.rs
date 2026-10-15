use crate::models::{
    access::{self, TenantTransaction},
    care::{locations, medications, people},
    errors::OperationError,
};
use serde_json::{Value, json};

pub async fn show(tenant: &TenantTransaction) -> Result<Value, OperationError> {
    access::recheck(tenant).await?;
    let manager = access::can_manage_household(tenant);
    let manage_locations = locations::can_manage(tenant).await?;
    let create_person = people::can_create(tenant).await?;
    let manage_ids = access::current_people(tenant)
        .await?
        .iter()
        .map(|person| person.id)
        .collect::<Vec<i64>>();
    let create_medication = medications::crud::can_create(tenant).await?;
    Ok(json!({"data": {
        "people": {"create": create_person, "manage_ids": manage_ids},
        "locations": {"create": manage_locations, "update": manage_locations},
        "medications": {"create": create_medication, "update": manager, "manage_stock": manager}
    }}))
}
