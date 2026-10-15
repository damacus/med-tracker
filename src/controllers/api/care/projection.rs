use super::*;
use crate::models::{
    care::{doses::DoseSource, take_history},
    ids::TakeId,
};

pub(super) fn take_etag(take: &medication_take::Model) -> String {
    let input = format!(
        "MedicationTake:{}:{}",
        take.id,
        take.updated_at.and_utc().timestamp_micros()
    );
    format!("\"{}\"", hex::encode(Sha256::digest(input.as_bytes())))
}

pub(super) async fn serialize(
    db: &DatabaseTransaction,
    takes: &[medication_take::Model],
    household_id: i64,
) -> Result<Vec<Value>, OperationError> {
    let ids = takes
        .iter()
        .map(|take| TakeId::from_database(take.id))
        .collect::<Vec<_>>();
    let records = take_history::by_ids(db, &ids).await?;
    if records.len() != takes.len()
        || records
            .iter()
            .any(|record| record.household_id.get() != household_id)
    {
        return Err(OperationError::Unavailable);
    }
    Ok(records.into_iter().map(|record| {
        let (schedule_id,schedule_portable_id,assignment_id,assignment_portable_id)=match record.source {
            DoseSource::Schedule(id)=>(Some(id.get()),Some(record.source_portable_id),None,None),
            DoseSource::Assignment(id)=>(None,None,Some(id.get()),Some(record.source_portable_id)),
        };
        json!({
            "id":record.id,"portable_id":record.portable_id,"client_uuid":record.client_uuid,
            "schedule_id":schedule_id,"schedule_portable_id":schedule_portable_id,
            "person_medication_id":assignment_id,"person_medication_portable_id":assignment_portable_id,
            "taken_from_medication_id":record.taken_from_medication_id,
            "taken_from_medication_portable_id":record.taken_from_medication_portable_id,
            "taken_from_location_id":record.taken_from_location_id,
            "taken_from_location_portable_id":record.taken_from_location_portable_id,
            "dose_amount":record.dose_amount.map(|amount| decimal_string(amount.to_string())),
            "dose_unit":record.dose_unit,"taken_at":record.taken_at.map(take_history::utc_timestamp),
            "updated_at":take_history::utc_timestamp(record.updated_at),
            "person_id":record.person_id,"person_portable_id":record.person_portable_id,
            "medication_id":record.medication_id,"medication_portable_id":record.medication_portable_id
        })
    }).collect())
}

fn decimal_string(value: String) -> String {
    if value.contains('.') {
        let trimmed = value.trim_end_matches('0');
        if trimmed.ends_with('.') {
            format!("{trimmed}0")
        } else {
            trimmed.into()
        }
    } else {
        format!("{value}.0")
    }
}
