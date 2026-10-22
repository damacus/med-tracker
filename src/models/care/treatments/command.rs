use super::*;
use loco_rs::model::TenantQueryExt;

pub(super) async fn apply_input(
    db: &DatabaseTransaction,
    context: &AuthContext,
    input: &ValidatedScheduleInput,
    fields: &mut ScheduleFields,
    existing: Option<&schedule::Model>,
) -> Result<Result<(), InputFailure>, ApiError> {
    let input = input.as_ref();
    if let Some(id) = input.person_id.as_ref() {
        let Some(person) = find_person(db, context, id.as_str()).await? else {
            return Ok(Err(InputFailure::NotFound));
        };
        if !person_access(db, context, person.id, false).await? {
            return Ok(Err(InputFailure::NotFound));
        }
        if existing.is_some_and(|existing| existing.person_id != person.id) {
            return Ok(Err(InputFailure::Invalid("person_id", "cannot be changed")));
        }
        fields.person_id = Some(person.id);
    }
    if let Some(id) = input.medication_id.as_ref() {
        let Some(medication) = visible_medication(db, context, id.as_str()).await? else {
            return Ok(Err(InputFailure::NotFound));
        };
        fields.medication_id = Some(medication.id);
    }
    if let Some(id) = input.source_dosage_option_id.as_ref() {
        let Some(option) = find_dosage(db, context, id.as_str()).await? else {
            return Ok(Err(InputFailure::NotFound));
        };
        fields.source_dosage_option_id = Some(option.id);
    }
    if let Some(amount) = input.dose_amount {
        fields.dose_amount = Some(amount);
    }
    if let Some(unit) = input.dose_unit.as_ref() {
        fields.dose_unit = Some(unit.clone());
    }
    if let Some(frequency) = input.frequency.as_ref() {
        fields.frequency = Some(frequency.clone());
    }
    if let Some(notes) = input.notes.as_ref() {
        fields.notes = Some(notes.clone());
    }
    if let Some(start) = input.start_date {
        fields.start_date = Some(start);
    }
    if let Some(end) = input.end_date {
        fields.end_date = Some(end);
    }
    if let Some(maximum) = input.max_daily_doses {
        fields.max_daily_doses = Some(maximum);
    }
    if let Some(minimum) = input.min_hours_between_doses {
        fields.min_hours_between_doses = minimum;
    }
    if let Some(cycle) = input.dose_cycle {
        fields.dose_cycle = Some(cycle.database());
    }
    if let Some(kind) = input.schedule_type {
        fields.schedule_type = kind.database();
    }
    if let Some(config) = input.schedule_config.as_ref() {
        fields.schedule_config = config.clone();
    }
    if let Some(option_id) = fields.source_dosage_option_id {
        let Some(option) = dosage::Entity::find_by_id(option_id).one(db).await? else {
            return Ok(Err(InputFailure::NotFound));
        };
        if Some(option.medication_id) != fields.medication_id
            || Some(option.amount) != fields.dose_amount
            || fields.dose_unit.as_deref() != Some(option.unit.as_str())
        {
            return Ok(Err(InputFailure::Invalid(
                "source_dosage_option",
                "must match the selected medication and dose",
            )));
        }
    } else if existing.is_none()
        && let (Some(medication_id), Some(amount), Some(unit)) = (
            fields.medication_id,
            fields.dose_amount,
            fields.dose_unit.as_deref(),
        )
    {
        let matches = dosage::Entity::find()
            .in_tenant(context.scope().household_id)
            .filter(dosage::Column::MedicationId.eq(medication_id))
            .filter(dosage::Column::Amount.eq(amount))
            .filter(dosage::Column::Unit.eq(unit))
            .all(db)
            .await?;
        if matches.len() == 1 {
            fields.source_dosage_option_id = Some(matches[0].id);
        }
    }
    Ok(Ok(()))
}
