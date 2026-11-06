use std::collections::{BTreeMap, HashSet};

use chrono_tz::Tz;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

use super::{CreatedRecord, import_invalid};
use crate::models::{
    access::{self, TenantTransaction},
    care::{person_schedules, sync},
    entities::{
        adoption_fact, medication, medication_take, pause_period, person_schedule, schedule,
        shared_administration, shared_execution, shared_schedule, sharing_approval, version,
    },
    errors::OperationError,
    identity::credentials::CredentialProvenance,
    ids::PersonId,
};

pub(crate) struct VerifiedCanonicalTake {
    pub(crate) take: medication_take::Model,
    pub(crate) capture: adoption_fact::Model,
    pub(crate) safety_index: shared_administration::Model,
}

pub(crate) struct CanonicalRestoredSchedule {
    pub(crate) execution: schedule::Model,
    pub(crate) plan: person_schedule::Model,
    pub(crate) shared_state: shared_schedule::Model,
    pub(crate) execution_link: shared_execution::Model,
    pub(crate) takes: Vec<VerifiedCanonicalTake>,
}

struct HistoryContext<'a> {
    plan_id: &'a str,
    source: &'a schedule::Model,
    actor_account: i64,
    actor_person: i64,
    actor_membership: i64,
    guardian_declaration: Option<i64>,
    request_id: &'a str,
    version_id: i64,
    authentication_method: &'a str,
    credential_reference: &'a str,
}

fn history_matches(
    context: &HistoryContext<'_>,
    take: &medication_take::Model,
    fact: &adoption_fact::Model,
    safety: &shared_administration::Model,
) -> bool {
    take.schedule_id == Some(context.source.id)
        && take.person_medication_id.is_none()
        && take.household_id == context.source.household_id
        && fact.medication_take_id == take.id
        && fact.person_schedule_id.as_deref() == Some(context.plan_id)
        && fact.source_schedule_id == context.source.id
        && fact.source_portable_id == context.source.portable_id
        && fact.person_id == context.source.person_id
        && fact.household_id == take.household_id
        && fact.take_portable_id == take.portable_id
        && fact.original_version_id == context.version_id
        && fact.client_uuid == take.client_uuid
        && Some(fact.taken_at) == take.taken_at
        && Some(fact.dose_amount) == take.dose_amount
        && fact.dose_unit == take.dose_unit
        && fact.taken_from_medication_id == take.taken_from_medication_id
        && fact.taken_from_location_id == take.taken_from_location_id
        && fact.take_created_at == take.created_at
        && fact.take_updated_at == take.updated_at
        && fact.fact_hash.len() == 32
        && fact.captured_by_account_id == context.actor_account
        && fact.captured_by_person_id == context.actor_person
        && fact.captured_by_membership_id == context.actor_membership
        && fact.guardian_declaration_id == context.guardian_declaration
        && fact.request_id == context.request_id
        && fact.authentication_method.as_deref() == Some(context.authentication_method)
        && fact.credential_reference.as_deref() == Some(context.credential_reference)
        && safety.person_schedule_id == context.plan_id
        && safety.household_id == take.household_id
        && safety.medication_take_id == take.id
        && Some(safety.taken_at) == take.taken_at
        && safety.occurrence_starts_on.is_none()
        && safety.occurrence_position.is_none()
}

fn clinical_matches(
    source: &schedule::Model,
    plan: &person_schedule::Model,
    shared: &shared_schedule::Model,
    zone: Tz,
) -> bool {
    plan.person_id == source.person_id
        && plan.origin_household_id == source.household_id
        && plan.origin_schedule_id == source.id
        && plan.origin_stock_id == source.medication_id
        && plan.created_at == source.created_at
        && plan.origin_time_zone == zone.name()
        && plan.timezone_provenance == "captured_sharer_zone"
        && plan.clinical_revision == 1
        && plan.active == source.active
        && plan.retired_at == source.retired_at
        && plan.dose_amount == source.dose_amount
        && plan.dose_unit == source.dose_unit
        && plan.dose_cycle == source.dose_cycle
        && plan.frequency == source.frequency
        && plan.start_date == source.start_date
        && plan.end_date == source.end_date
        && plan.notes == source.notes
        && plan.max_daily_doses == source.max_daily_doses
        && plan.min_hours_between_doses == source.min_hours_between_doses
        && plan.schedule_type == source.schedule_type
        && plan.schedule_config == source.schedule_config
        && shared.id == plan.id
        && shared.person_id == plan.person_id
        && shared.created_at == plan.created_at
        && shared.origin_time_zone == plan.origin_time_zone
        && shared.clinical_revision == plan.clinical_revision
        && shared.active == plan.active
        && shared.retired_at == plan.retired_at
        && shared.dose_amount == plan.dose_amount
        && shared.dose_unit == plan.dose_unit
        && shared.dose_cycle == plan.dose_cycle
        && shared.frequency == plan.frequency
        && shared.start_date == plan.start_date
        && shared.end_date == plan.end_date
        && shared.max_daily_doses == plan.max_daily_doses
        && shared.min_hours_between_doses == plan.min_hours_between_doses
        && shared.schedule_type == plan.schedule_type
        && shared.schedule_config == plan.schedule_config
        && shared.pause_started_at == plan.pause_started_at
        && shared.pause_instance_id == plan.pause_instance_id
        && shared.medicine_name == plan.medicine_name
        && shared.dmd_system == plan.dmd_system
        && shared.dmd_code == plan.dmd_code
        && shared.dmd_concept_class == plan.dmd_concept_class
        && shared.stock_unit == plan.stock_unit
        && shared.stock_dose_default == plan.stock_dose_default
        && shared.stock_default_known == plan.stock_default_known
}

async fn local_create_version(
    tenant: &TenantTransaction,
    kind: &str,
    id: i64,
) -> Result<version::Model, OperationError> {
    let original = version::Entity::find()
        .filter(version::Column::ItemType.eq(kind))
        .filter(version::Column::ItemId.eq(id))
        .filter(version::Column::Event.eq("create"))
        .order_by_asc(version::Column::Id)
        .one(tenant.transaction())
        .await?
        .ok_or_else(|| import_invalid("Local restore audit is unavailable"))?;
    if original.household_id != Some(tenant.scope().household_id)
        || original.actor_membership_id != Some(tenant.membership().id)
        || original.whodunnit.as_deref()
            != Some(tenant.scope().actor.account_id.to_string().as_str())
        || original.request_id.as_deref() != Some(tenant.scope().request_id.as_str())
    {
        return Err(import_invalid("Local restore audit actor does not match"));
    }
    Ok(original)
}

pub(crate) async fn initialize_canonical_schedules(
    tenant: &TenantTransaction,
    created: &[CreatedRecord],
    source_zone: Tz,
    eligibility_zone: Tz,
    provenance: &CredentialProvenance,
) -> Result<Vec<CanonicalRestoredSchedule>, OperationError> {
    let mut schedules = BTreeMap::new();
    let mut takes = BTreeMap::new();
    for record in created {
        match record {
            CreatedRecord::Schedule(source) => {
                if schedules.insert(source.id, source.as_ref()).is_some() {
                    return Err(import_invalid("Duplicate imported schedule"));
                }
            }
            CreatedRecord::Dose(take) => {
                if takes.insert(take.id, take.as_ref()).is_some() {
                    return Err(import_invalid("Duplicate imported take"));
                }
            }
            _ => {}
        }
    }
    if takes.values().any(|take| {
        take.schedule_id
            .is_some_and(|id| !schedules.contains_key(&id))
    }) {
        return Err(import_invalid("Imported take source is missing"));
    }
    if schedules.is_empty() {
        return Ok(Vec::new());
    }
    sync::lock(tenant).await?;
    let db = tenant.transaction();
    let sources = schedule::Entity::find()
        .filter(schedule::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(schedule::Column::Id.is_in(schedules.keys().copied()))
        .order_by_asc(schedule::Column::Id)
        .lock_exclusive()
        .all(db)
        .await?;
    if sources.len() != schedules.len() {
        return Err(import_invalid("Imported schedule is unavailable"));
    }
    let mut seen_takes = HashSet::new();
    let mut work = Vec::new();
    for source in sources {
        let expected = schedules
            .get(&source.id)
            .ok_or(OperationError::Unavailable)?;
        let mut earlier = (**expected).clone();
        earlier.active = source.active;
        earlier.updated_at = source.updated_at;
        if earlier != source
            || source.person_schedule_id.is_some()
            || source.canonical_origin
            || source.execution_ended_at.is_some()
            || source.sharing_approval_id.is_some()
            || source.origin_time_zone.is_some()
            || source.clinical_revision.is_some()
        {
            return Err(import_invalid(
                "Imported schedule changed before canonical initialization",
            ));
        }
        local_create_version(tenant, "Schedule", source.id).await?;
        if !person_schedules::sharing::clinical_authority(
            db,
            PersonId::from_database(source.person_id),
        )
        .await?
        {
            return Err(import_invalid(
                "Canonical schedule requires current self or guardian authority",
            ));
        }
        let guardian = if tenant.membership().person_id == source.person_id {
            None
        } else {
            Some(
                person_schedules::sharing::guardian(db, PersonId::from_database(source.person_id))
                    .await?
                    .ok_or_else(|| {
                        import_invalid(
                            "Canonical schedule requires current self or guardian authority",
                        )
                    })?,
            )
        };
        let source_takes = medication_take::Entity::find()
            .filter(medication_take::Column::HouseholdId.eq(source.household_id))
            .filter(medication_take::Column::ScheduleId.eq(source.id))
            .order_by_asc(medication_take::Column::Id)
            .lock_shared()
            .all(db)
            .await?;
        for take in &source_takes {
            if takes.get(&take.id).is_none_or(|expected| *expected != take) {
                return Err(import_invalid(
                    "Imported take changed before canonical initialization",
                ));
            }
            seen_takes.insert(take.id);
            local_create_version(tenant, "MedicationTake", take.id).await?;
        }
        let pauses = pause_period::Entity::find()
            .filter(pause_period::Column::HouseholdId.eq(source.household_id))
            .filter(pause_period::Column::ScheduleId.eq(source.id))
            .filter(pause_period::Column::EndedAt.is_null())
            .lock_shared()
            .all(db)
            .await?;
        if pauses.len() > 1 || (source.active && !pauses.is_empty()) {
            return Err(import_invalid("Imported schedule pause state conflicts"));
        }
        let stock = medication::Entity::find_by_id(source.medication_id)
            .filter(medication::Column::HouseholdId.eq(source.household_id))
            .one(db)
            .await?
            .ok_or_else(|| import_invalid("Imported schedule stock is unavailable"))?;
        work.push((
            source,
            source_takes,
            pauses.into_iter().next(),
            stock,
            guardian,
        ));
    }
    if takes
        .values()
        .any(|take| take.schedule_id.is_some() && !seen_takes.contains(&take.id))
    {
        return Err(import_invalid("Imported schedule take is unavailable"));
    }
    let mut output = Vec::with_capacity(work.len());
    for (source, source_takes, pause, stock, guardian) in work {
        let linked = person_schedules::save_execution(
            tenant,
            source.clone(),
            Some(source_zone),
            eligibility_zone,
            Some(provenance),
        )
        .await?;
        let linked = schedule::Entity::find_by_id(linked.id)
            .one(db)
            .await?
            .ok_or(OperationError::Unavailable)?;
        let plan_id = linked
            .person_schedule_id
            .as_deref()
            .ok_or(OperationError::Unavailable)?;
        let plan = person_schedule::Entity::find_by_id(plan_id)
            .one(db)
            .await?
            .ok_or(OperationError::Unavailable)?;
        let shared = shared_schedule::Entity::find_by_id(plan_id)
            .one(db)
            .await?
            .ok_or(OperationError::Unavailable)?;
        let execution_link = shared_execution::Entity::find_by_id(source.id)
            .one(db)
            .await?
            .ok_or(OperationError::Unavailable)?;
        let approval = sharing_approval::Entity::find()
            .filter(sharing_approval::Column::HouseholdId.eq(source.household_id))
            .filter(sharing_approval::Column::PersonId.eq(source.person_id))
            .filter(sharing_approval::Column::EndedAt.is_null())
            .one(db)
            .await?;
        let mut before = source.clone();
        before.person_schedule_id = linked.person_schedule_id.clone();
        before.canonical_origin = linked.canonical_origin;
        before.clinical_revision = linked.clinical_revision;
        before.origin_time_zone = linked.origin_time_zone.clone();
        if linked != before
            || !linked.canonical_origin
            || linked.origin_time_zone.as_deref() != Some(source_zone.name())
            || linked.clinical_revision != Some(plan.clinical_revision)
            || execution_link.schedule_id != source.id
            || execution_link.portable_id != source.portable_id
            || execution_link.household_id != source.household_id
            || execution_link.person_id != source.person_id
            || execution_link.person_schedule_id != plan.id
            || execution_link.approval_id != approval.map(|row| row.id)
            || !clinical_matches(&source, &plan, &shared, source_zone)
        {
            return Err(import_invalid("Canonical schedule verification failed"));
        }
        let stock_after = medication::Entity::find_by_id(stock.id)
            .one(db)
            .await?
            .ok_or(OperationError::Unavailable)?;
        let stock_default = stock
            .dose_amount
            .and_then(|amount| amount.to_string().parse().ok());
        if stock_after != stock
            || plan.origin_stock_portable_id != stock.portable_id
            || plan.medicine_name != stock.name
            || plan.dmd_system != stock.dmd_system
            || plan.dmd_code != stock.dmd_code
            || plan.dmd_concept_class != stock.dmd_concept_class
            || plan.stock_unit != stock.dose_unit
            || plan.stock_dose_default != stock_default
            || plan.stock_default_known != (stock.dose_amount.is_none() || stock_default.is_some())
        {
            return Err(import_invalid(
                "Canonical stock snapshot verification failed",
            ));
        }
        if let Some(period) = pause {
            let actual = pause_period::Entity::find_by_id(period.id)
                .one(db)
                .await?
                .ok_or(OperationError::Unavailable)?;
            let mut expected = period.clone();
            expected.canonical_person_schedule_id = Some(plan.id.clone());
            expected.canonical_pause_revision = Some(plan.clinical_revision);
            if actual != expected
                || plan.pause_instance_id.as_deref() != Some(period.portable_id.as_str())
                || plan.pause_started_at != period.started_at
                || plan.pause_reason.as_deref() != Some(period.reason.as_str())
                || plan.pause_note != period.note
            {
                return Err(import_invalid("Canonical pause verification failed"));
            }
        } else if plan.pause_instance_id.is_some()
            || plan.pause_started_at.is_some()
            || plan.pause_reason.is_some()
            || plan.pause_note.is_some()
        {
            return Err(import_invalid("Canonical pause verification failed"));
        }
        let mut verified = Vec::with_capacity(source_takes.len());
        let captures = adoption_fact::Entity::find()
            .filter(adoption_fact::Column::PersonScheduleId.eq(&plan.id))
            .all(db)
            .await?;
        let safety_rows = shared_administration::Entity::find()
            .filter(shared_administration::Column::PersonScheduleId.eq(&plan.id))
            .all(db)
            .await?;
        if captures.len() != source_takes.len() || safety_rows.len() != source_takes.len() {
            return Err(import_invalid(
                "Imported take history is incomplete or duplicated",
            ));
        }
        for take in source_takes {
            let original = local_create_version(tenant, "MedicationTake", take.id).await?;
            let fact = adoption_fact::Entity::find_by_id(take.id)
                .one(db)
                .await?
                .ok_or_else(|| import_invalid("Imported take capture is missing"))?;
            let safety = shared_administration::Entity::find()
                .filter(shared_administration::Column::PersonScheduleId.eq(&plan.id))
                .filter(shared_administration::Column::MedicationTakeId.eq(take.id))
                .one(db)
                .await?
                .ok_or_else(|| import_invalid("Imported take safety index is missing"))?;
            let current = medication_take::Entity::find_by_id(take.id)
                .one(db)
                .await?
                .ok_or(OperationError::Unavailable)?;
            let context = HistoryContext {
                plan_id: &plan.id,
                source: &source,
                actor_account: tenant.scope().actor.account_id,
                actor_person: tenant.membership().person_id,
                actor_membership: tenant.membership().id,
                guardian_declaration: guardian,
                request_id: &tenant.scope().request_id,
                version_id: original.id,
                authentication_method: provenance.method.label(),
                credential_reference: &provenance.reference,
            };
            if current != take || !history_matches(&context, &take, &fact, &safety) {
                return Err(import_invalid("Imported take capture verification failed"));
            }
            verified.push(VerifiedCanonicalTake {
                take,
                capture: fact,
                safety_index: safety,
            });
        }
        if shared.latest_taken_at
            != verified
                .iter()
                .filter_map(|entry| entry.take.taken_at)
                .max()
        {
            return Err(import_invalid("Canonical latest take verification failed"));
        }
        if !person_schedules::sharing::clinical_authority(
            db,
            PersonId::from_database(source.person_id),
        )
        .await?
            || (guardian.is_some()
                && person_schedules::sharing::guardian(
                    db,
                    PersonId::from_database(source.person_id),
                )
                .await?
                    != guardian)
        {
            return Err(import_invalid(
                "Canonical schedule requires current self or guardian authority",
            ));
        }
        output.push(CanonicalRestoredSchedule {
            execution: linked,
            plan,
            shared_state: shared,
            execution_link,
            takes: verified,
        });
    }
    access::recheck(tenant).await?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use sea_orm::prelude::Decimal;

    #[test]
    fn capture_and_safety_index_must_match_source_take_and_actor() {
        let time = NaiveDate::from_ymd_opt(2020, 1, 2)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap();
        let source = schedule::Model {
            id: 3,
            household_id: 4,
            person_id: 5,
            medication_id: 6,
            portable_id: "schedule".into(),
            active: true,
            created_at: time,
            updated_at: time,
            dose_amount: Some(Decimal::ONE),
            dose_cycle: None,
            dose_unit: Some("tablet".into()),
            end_date: None,
            frequency: None,
            max_daily_doses: None,
            min_hours_between_doses: None,
            notes: None,
            person_schedule_id: None,
            sharing_approval_id: None,
            canonical_origin: false,
            clinical_revision: None,
            origin_time_zone: None,
            execution_ended_at: None,
            retired_at: None,
            schedule_config: serde_json::json!({}),
            schedule_type: 0,
            source_dosage_option_id: Some(7),
            start_date: None,
        };
        let take = medication_take::Model {
            id: 8,
            household_id: 4,
            schedule_id: Some(3),
            person_medication_id: None,
            portable_id: "take".into(),
            client_uuid: None,
            created_at: time,
            updated_at: time,
            taken_at: Some(time),
            dose_amount: Some(Decimal::ONE),
            dose_unit: Some("tablet".into()),
            taken_from_medication_id: Some(6),
            taken_from_location_id: Some(9),
        };
        let fact = adoption_fact::Model {
            medication_take_id: 8,
            person_schedule_id: Some("plan".into()),
            source_schedule_id: 3,
            source_portable_id: "schedule".into(),
            person_id: 5,
            household_id: 4,
            take_portable_id: "take".into(),
            original_version_id: 10,
            client_uuid: None,
            taken_at: time,
            dose_amount: Decimal::ONE,
            dose_unit: Some("tablet".into()),
            taken_from_medication_id: Some(6),
            taken_from_location_id: Some(9),
            take_created_at: time,
            take_updated_at: time,
            fact_hash: vec![1; 32],
            captured_by_account_id: 11,
            captured_by_person_id: 5,
            captured_by_membership_id: 12,
            guardian_declaration_id: None,
            request_id: "request".into(),
            authentication_method: Some("test".into()),
            credential_reference: Some("credential".into()),
            captured_at: time,
        };
        let safety = shared_administration::Model {
            id: 13,
            person_schedule_id: "plan".into(),
            household_id: 4,
            medication_take_id: 8,
            taken_at: time,
            occurrence_starts_on: None,
            occurrence_position: None,
        };
        let context = HistoryContext {
            plan_id: "plan",
            source: &source,
            actor_account: 11,
            actor_person: 5,
            actor_membership: 12,
            guardian_declaration: None,
            request_id: "request",
            version_id: 10,
            authentication_method: "test",
            credential_reference: "credential",
        };
        assert!(history_matches(&context, &take, &fact, &safety));
        let mut bad_fact = fact.clone();
        bad_fact.taken_from_location_id = Some(99);
        assert!(!history_matches(&context, &take, &bad_fact, &safety));
        bad_fact = fact.clone();
        bad_fact.captured_by_account_id = 99;
        assert!(!history_matches(&context, &take, &bad_fact, &safety));
        bad_fact = fact.clone();
        bad_fact.dose_amount = Decimal::from(2);
        assert!(!history_matches(&context, &take, &bad_fact, &safety));
        bad_fact = fact.clone();
        bad_fact.source_schedule_id = 99;
        assert!(!history_matches(&context, &take, &bad_fact, &safety));
        bad_fact = fact.clone();
        bad_fact.taken_at = time + chrono::Duration::seconds(1);
        assert!(!history_matches(&context, &take, &bad_fact, &safety));
        let mut bad_safety = safety.clone();
        bad_safety.person_schedule_id = "other".into();
        assert!(!history_matches(&context, &take, &fact, &bad_safety));
        bad_safety = safety;
        bad_safety.medication_take_id = 99;
        assert!(!history_matches(&context, &take, &fact, &bad_safety));
    }
}
