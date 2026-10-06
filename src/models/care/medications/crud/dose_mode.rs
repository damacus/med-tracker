use super::*;
pub(super) async fn sync_single_dose_mode(
    context: &StockContext<'_>,
    medication_id: i64,
) -> Result<(), ApiError> {
    let db = context.tenant.transaction();
    let request_id = &context.tenant.scope().request_id;
    let sources = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(context.tenant.scope().household_id))
        .filter(person_medication::Column::MedicationId.eq(medication_id))
        .filter(person_medication::Column::SourceDosageOptionId.is_not_null())
        .all(db)
        .await
        .map_err(database_error)?;
    let options = dosage::Entity::find()
        .filter(dosage::Column::HouseholdId.eq(context.tenant.scope().household_id))
        .filter(dosage::Column::MedicationId.eq(medication_id))
        .all(db)
        .await
        .map_err(database_error)?;
    let person_ids: Vec<i64> = sources.iter().map(|source| source.person_id).collect();
    let people = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(context.tenant.scope().household_id))
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
        .filter(dosage::Column::HouseholdId.eq(context.tenant.scope().household_id))
        .filter(dosage::Column::MedicationId.eq(medication_id))
        .exec(db)
        .await
        .map_err(database_error)?;
    for option in options {
        api_tombstone::ActiveModel {
            household_id: Set(context.tenant.scope().household_id),
            household_membership_id: Set(Some(context.tenant.membership().id)),
            account_id: Set(Some(context.tenant.scope().actor.account_id)),
            action: Set("delete".to_owned()),
            record_type: Set("MedicationDosageOption".to_owned()),
            record_portable_id: Set(option.portable_id.clone()),
            metadata: Set(json!({
                "record_type": "MedicationDosageOption",
                "record_id": option.id,
                "portable_id": option.portable_id,
                "medication_id": medication_id
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
            household_id: Set(context.tenant.scope().household_id),
            household_membership_id: Set(Some(context.tenant.membership().id)),
            account_id: Set(Some(context.tenant.scope().actor.account_id)),
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
