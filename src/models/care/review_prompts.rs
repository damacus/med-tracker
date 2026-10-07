mod evidence;

use crate::models::{
    access::TenantTransaction,
    entities::{medication, person_medication, review_evidence, review_prompt, schedule},
    errors::OperationError,
};
use chrono::{NaiveDate, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

fn display_name(record: &medication::Model) -> String {
    record
        .friendly_name
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .or(record.name.as_deref())
        .unwrap_or("")
        .to_owned()
}

pub fn value(record: &review_prompt::Model) -> Value {
    json!({
        "id": record.id.to_string(),
        "person_id": record.person_id.to_string(),
        "primary_medication_id": record.primary_medication_id.to_string(),
        "interacting_medication_id": record.interacting_medication_id.to_string(),
        "evidence_record_id": record.evidence_record_id.to_string(),
        "primary_medication_name": record.primary_medication_name,
        "interacting_medication_name": record.interacting_medication_name,
        "evidence_source_name": record.evidence_source_name,
        "evidence_source_url": record.evidence_source_url,
        "evidence_source_version": record.evidence_source_version,
        "matched_term": record.matched_term,
        "match_type": record.match_type,
        "source_instruction": record.source_instruction,
        "match_reason": record.match_reason,
        "evidence_text": record.evidence_text,
        "etag": crate::models::care::sync::review_prompts::tag(record),
        "evidence_source_checked_on": record.evidence_source_checked_on.to_string(),
        "evidence_source_effective_on": record.evidence_source_effective_on.to_string(),
        "risk_level": record.risk_level,
        "match_confidence": record.match_confidence,
        "status": record.status,
        "practitioner_name": record.practitioner_name,
        "practitioner_role": record.practitioner_role,
        "review_note": record.review_note,
        "reviewed_on": record.reviewed_on.map(|date| date.to_string()),
        "reviewed_by_membership_id": record.reviewed_by_membership_id.map(|id| id.to_string()),
        "updated_at": record.updated_at.and_utc().to_rfc3339(),
    })
}

pub async fn refresh(
    tenant: &TenantTransaction,
    person_id: i64,
    today: NaiveDate,
) -> Result<(), OperationError> {
    refresh_many(tenant, &[person_id], today).await
}

pub async fn refresh_many(
    tenant: &TenantTransaction,
    person_ids: &[i64],
    today: NaiveDate,
) -> Result<(), OperationError> {
    if person_ids.is_empty() {
        return Ok(());
    }
    let db = tenant.transaction();
    let household_id = tenant.scope().household_id;
    let assignments = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(household_id))
        .filter(person_medication::Column::PersonId.is_in(person_ids.to_vec()))
        .filter(person_medication::Column::Active.eq(true))
        .filter(person_medication::Column::RetiredAt.is_null())
        .all(db)
        .await?;
    let schedules = schedule::Entity::find()
        .filter(schedule::Column::HouseholdId.eq(household_id))
        .filter(schedule::Column::PersonId.is_in(person_ids.to_vec()))
        .filter(schedule::Column::Active.eq(true))
        .filter(schedule::Column::RetiredAt.is_null())
        .filter(schedule::Column::StartDate.lte(today))
        .filter(schedule::Column::EndDate.gte(today))
        .all(db)
        .await?;
    let all_medication_ids: Vec<i64> = assignments
        .iter()
        .map(|row| row.medication_id)
        .chain(schedules.iter().map(|row| row.medication_id))
        .collect();
    let mut all_medication_ids = all_medication_ids;
    all_medication_ids.sort_unstable();
    all_medication_ids.dedup();
    if all_medication_ids.len() < 2 {
        return Ok(());
    }
    let medications: HashMap<i64, medication::Model> = medication::Entity::find()
        .filter(medication::Column::HouseholdId.eq(household_id))
        .filter(medication::Column::Id.is_in(all_medication_ids))
        .all(db)
        .await?
        .into_iter()
        .map(|row| (row.id, row))
        .collect();
    let evidence = review_evidence::Entity::find()
        .filter(review_evidence::Column::MatchStatus.is_in(["unreviewed", "reviewed_pair"]))
        .order_by_asc(review_evidence::Column::Id)
        .all(db)
        .await?;
    let mut existing: HashSet<(i64, i64, i64, i64)> = review_prompt::Entity::find()
        .filter(review_prompt::Column::HouseholdId.eq(household_id))
        .filter(review_prompt::Column::PersonId.is_in(person_ids.to_vec()))
        .all(db)
        .await?
        .into_iter()
        .map(|row| {
            (
                row.person_id,
                row.primary_medication_id,
                row.interacting_medication_id,
                row.evidence_record_id,
            )
        })
        .collect();
    for &person_id in person_ids {
        let mut medication_ids: Vec<i64> = assignments
            .iter()
            .filter(|row| row.person_id == person_id)
            .map(|row| row.medication_id)
            .chain(
                schedules
                    .iter()
                    .filter(|row| row.person_id == person_id)
                    .map(|row| row.medication_id),
            )
            .collect();
        medication_ids.sort_unstable();
        medication_ids.dedup();
        for (first_index, first_id) in medication_ids.iter().enumerate() {
            for second_id in medication_ids.iter().skip(first_index + 1) {
                let (Some(first), Some(second)) =
                    (medications.get(first_id), medications.get(second_id))
                else {
                    continue;
                };
                let first_name = display_name(first);
                let second_name = display_name(second);
                for matched in evidence::matches(&first_name, &second_name, &evidence) {
                    let (primary, interacting, primary_name, interacting_name) =
                        if matched.primary_is_first {
                            (first, second, &first_name, &second_name)
                        } else {
                            (second, first, &second_name, &first_name)
                        };
                    if !existing.insert((
                        person_id,
                        primary.id,
                        interacting.id,
                        matched.evidence.id,
                    )) {
                        continue;
                    }
                    let now = Utc::now().naive_utc();
                    review_prompt::ActiveModel {
                        household_id: Set(household_id),
                        person_id: Set(person_id),
                        primary_medication_id: Set(primary.id),
                        interacting_medication_id: Set(interacting.id),
                        evidence_record_id: Set(matched.evidence.id),
                        primary_medication_name: Set(primary_name.clone()),
                        interacting_medication_name: Set(interacting_name.clone()),
                        evidence_source_name: Set(matched.evidence.source_name.clone()),
                        evidence_source_url: Set(matched.evidence.source_url.clone()),
                        evidence_source_checked_on: Set(matched.evidence.retrieved_on),
                        evidence_source_effective_on: Set(matched
                            .evidence
                            .source_effective_on
                            .unwrap_or(matched.evidence.retrieved_on)),
                        evidence_source_version: Set(matched
                            .evidence
                            .source_version
                            .clone()
                            .unwrap_or_else(|| "unknown".to_owned())),
                        evidence_text: Set(matched.excerpt),
                        matched_term: Set(matched.matched_term),
                        match_type: Set(matched.match_type.to_owned()),
                        source_instruction: Set(matched.instruction.to_owned()),
                        match_reason: Set(matched.reason),
                        risk_level: Set(matched.risk.clone()),
                        match_confidence: Set(matched.confidence.clone()),
                        status: Set(if matched.risk == "low" || matched.confidence == "low" {
                            "hidden_low_signal"
                        } else {
                            "needs_review"
                        }
                        .to_owned()),
                        created_at: Set(now),
                        updated_at: Set(now),
                        ..Default::default()
                    }
                    .insert(db)
                    .await?;
                }
            }
        }
    }
    Ok(())
}
