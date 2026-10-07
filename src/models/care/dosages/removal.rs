use super::*;
use crate::models::entities::{api_tombstone, person_medication, schedule};

pub async fn destroy(
    tenant: &TenantTransaction,
    id: &str,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    authorize(tenant).await?;
    let found = record(tenant, id, false).await?;
    let parent = parent(tenant, &found.medication_id.to_string(), true).await?;
    access::recheck(tenant).await?;
    let row = record(tenant, id, true).await?;
    let household_id = tenant.scope().household_id;
    let references = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(household_id))
        .filter(person_medication::Column::SourceDosageOptionId.eq(row.id))
        .count(tenant.transaction())
        .await?;
    let schedules = schedule::Entity::find()
        .filter(schedule::Column::HouseholdId.eq(household_id))
        .filter(schedule::Column::SourceDosageOptionId.eq(row.id))
        .count(tenant.transaction())
        .await?;
    if references > 0 || schedules > 0 {
        return Err(OperationError::Validation {
            details: json!({"errors":{"dosage_option":["is referenced by a schedule or medication assignment"]}}),
        });
    }
    let now = Utc::now().naive_utc();
    let savepoint = tenant.transaction().begin().await?;
    let result=async {
        dosage::Entity::delete_by_id(row.id).filter(dosage::Column::HouseholdId.eq(household_id)).exec(&savepoint).await?;
        if row.current_supply.is_some() {
            let before=super::super::medications::medication_snapshot(&parent);
            let parent=inventory::synchronize_inventory(&savepoint,parent,now,false).await?.ok_or_else(invalid)?;
            administration::persistence::record_version_as(tenant,"Medication",parent.id,"update",Some(before),super::super::medications::medication_snapshot(&parent),provenance).await?;
            persistence::sync(tenant,"Medication",parent.id,&parent.portable_id,"update",None).await?;
        }
        administration::persistence::record_version_as(tenant,"MedicationDosageOption",row.id,"destroy",Some(persistence::dosage_snapshot(&row)),json!({}),provenance).await?;
        api_tombstone::ActiveModel{
            household_id:Set(household_id),household_membership_id:Set(Some(tenant.membership().id)),account_id:Set(Some(tenant.scope().actor.account_id)),action:Set("delete".into()),
            record_type:Set("MedicationDosageOption".into()),record_portable_id:Set(row.portable_id.clone()),metadata:Set(json!({"record_type":"MedicationDosageOption","record_id":row.id,"portable_id":row.portable_id,"medication_id":row.medication_id})),deleted_at:Set(now),created_at:Set(now),updated_at:Set(now),..Default::default()
        }.insert(&savepoint).await?;
        persistence::sync(tenant,"MedicationDosageOption",row.id,&row.portable_id,"delete",Some(row.medication_id)).await?;
        Ok::<(),OperationError>(())
    }.await;
    match result {
        Ok(()) => {
            savepoint.commit().await?;
            Ok(())
        }
        Err(error) => {
            savepoint.rollback().await?;
            Err(error)
        }
    }
}
