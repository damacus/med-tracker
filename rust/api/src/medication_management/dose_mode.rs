use crate::database_error;
use crate::entities::api_change_event;
use crate::entities::api_tombstone;
use crate::entities::dosage;
use crate::entities::person;
use crate::entities::person_medication;
use crate::ApiError;
use crate::AuthContext;
use chrono::Utc;
use sea_orm::ActiveModelTrait;
use sea_orm::ColumnTrait;
use sea_orm::DatabaseTransaction;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::Set;
use serde_json::json;
use std::collections::HashMap;

pub(super) async fn sync_single_dose_mode(
    db: &DatabaseTransaction,
    context: &AuthContext,
    medication_id: i64,
    request_id: &str,
) -> Result<(), ApiError> {
    let sources = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(context.membership.household_id))
        .filter(person_medication::Column::MedicationId.eq(medication_id))
        .filter(person_medication::Column::SourceDosageOptionId.is_not_null())
        .all(db)
        .await
        .map_err(database_error)?;
    let options = dosage::Entity::find()
        .filter(dosage::Column::HouseholdId.eq(context.membership.household_id))
        .filter(dosage::Column::MedicationId.eq(medication_id))
        .all(db)
        .await
        .map_err(database_error)?;
    let person_ids: Vec<i64> = sources.iter().map(|source| source.person_id).collect();
    let people = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(context.membership.household_id))
        .filter(person::Column::Id.is_in(person_ids))
        .all(db)
        .await
        .map_err(database_error)?;
    let person_portable_ids: HashMap<i64, String> = people
        .into_iter()
        .map(|record| (record.id, record.portable_id))
        .collect();
    let now = Utc::now().naive_utc();
    for source in &sources {
        let mut active: person_medication::ActiveModel = source.clone().into();
        active.source_dosage_option_id = Set(None);
        active.updated_at = Set(now);
        active.update(db).await.map_err(database_error)?;
    }
    dosage::Entity::delete_many()
        .filter(dosage::Column::HouseholdId.eq(context.membership.household_id))
        .filter(dosage::Column::MedicationId.eq(medication_id))
        .exec(db)
        .await
        .map_err(database_error)?;
    for option in options {
        api_tombstone::ActiveModel {
            household_id: Set(context.membership.household_id),
            household_membership_id: Set(Some(context.membership.id)),
            account_id: Set(Some(context.account_id)),
            action: Set("delete".to_owned()),
            record_type: Set("MedicationDosageOption".to_owned()),
            record_portable_id: Set(option.portable_id.clone()),
            metadata: Set(json!({
                "record_type": "MedicationDosageOption",
                "record_id": option.id,
                "portable_id": option.portable_id
            })),
            deleted_at: Set(now),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(database_error)?;
    }
    for source in sources {
        api_change_event::ActiveModel {
            household_id: Set(context.membership.household_id),
            household_membership_id: Set(Some(context.membership.id)),
            account_id: Set(Some(context.account_id)),
            action: Set("update".to_owned()),
            record_type: Set("PersonMedication".to_owned()),
            record_id: Set(source.id),
            record_portable_id: Set(Some(source.portable_id.clone())),
            request_id: Set(Some(request_id.to_owned())),
            metadata: Set(json!({
                "record_type": "PersonMedication",
                "record_id": source.id,
                "portable_id": source.portable_id,
                "person_portable_id": person_portable_ids.get(&source.person_id)
            })),
            occurred_at: Set(now),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(database_error)?;
    }
    Ok(())
}
