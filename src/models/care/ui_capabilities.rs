use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    care::{locations, medications, people},
    errors::OperationError,
};
use crate::views::{
    DataResponse,
    capabilities::{
        LocationUiCapabilities, MedicationUiCapabilities, PeopleUiCapabilities, UiCapabilities,
    },
};

pub(crate) async fn show(
    tenant: &TenantTransaction,
) -> Result<DataResponse<UiCapabilities>, OperationError> {
    access::recheck(tenant).await?;
    let manager = access::can_manage_household(tenant);
    let manage_locations = locations::can_manage(tenant).await?;
    let create_person = people::can_create(tenant).await?;
    let manage_ids = access::people_with_access(tenant, PersonAccess::Manage)
        .await?
        .iter()
        .map(|person| person.id)
        .collect::<Vec<i64>>();
    let create_medication = medications::crud::can_create(tenant).await?;
    Ok(DataResponse {
        data: UiCapabilities {
            people: PeopleUiCapabilities {
                create: create_person,
                manage_ids,
            },
            locations: LocationUiCapabilities {
                create: manage_locations,
                update: manage_locations,
            },
            medications: MedicationUiCapabilities {
                create: create_medication,
                update: manager,
                manage_stock: manager,
            },
        },
    })
}
