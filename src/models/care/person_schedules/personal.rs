use super::*;
use crate::models::{
    access,
    care::{doses::CredentialProvenance, treatments},
    entities::{api_change_event, dosage, household, membership, pause_period, version},
    ids::AccountId,
};
use sea_orm::{
    ConnectionTrait, QueryOrder,
    sea_query::{Expr, Func, Query},
};
use serde::Serialize;
use serde_json::json;

#[derive(Serialize)]
pub(crate) struct PersonalPlan {
    pub(crate) id: String,
    pub(crate) clinical_revision: i64,
    pub(crate) etag: String,
    pub(crate) origin_time_zone: String,
    pub(crate) timezone_provenance: String,
    pub(crate) medicine_name: Option<String>,
    pub(crate) dmd_system: Option<String>,
    pub(crate) dmd_code: Option<String>,
    pub(crate) dmd_concept_class: Option<String>,
    pub(crate) stock_unit: Option<String>,
    pub(crate) pause_started_at: Option<chrono::DateTime<Utc>>,
    pub(crate) pause_reason: Option<String>,
    pub(crate) pause_note: Option<String>,
    pub(crate) active: bool,
    pub(crate) retired_at: Option<chrono::DateTime<Utc>>,
    pub(crate) dose_amount: Option<String>,
    pub(crate) dose_unit: Option<String>,
    pub(crate) frequency: Option<String>,
    pub(crate) start_date: Option<chrono::NaiveDate>,
    pub(crate) end_date: Option<chrono::NaiveDate>,
    pub(crate) notes: Option<String>,
    pub(crate) max_daily_doses: Option<i32>,
    pub(crate) min_hours_between_doses: Option<String>,
    pub(crate) dose_cycle: Option<crate::models::care::treatment_command::DoseCycle>,
    pub(crate) schedule_type: crate::models::care::treatment_command::ScheduleType,
    pub(crate) schedule_config: serde_json::Value,
}

impl PersonalPlan {
    fn from_record(row: person_schedule::Model) -> Result<Self, OperationError> {
        let identity = identity(&row)?;
        let _person = identity.person_id;
        Ok(Self {
            id: identity.id.as_str().into(),
            clinical_revision: identity.revision.get(),
            etag: format!(
                "\"person-schedule:{}:{}\"",
                identity.id.as_str(),
                identity.revision.get()
            ),
            origin_time_zone: identity.zone.name().into(),
            timezone_provenance: row.timezone_provenance,
            medicine_name: row.medicine_name,
            dmd_system: row.dmd_system,
            dmd_code: row.dmd_code,
            dmd_concept_class: row.dmd_concept_class,
            stock_unit: row.stock_unit,
            pause_started_at: row.pause_started_at.map(|time| time.and_utc()),
            pause_reason: row.pause_reason,
            pause_note: row.pause_note,
            active: row.active,
            retired_at: row.retired_at.map(|time| time.and_utc()),
            dose_amount: row.dose_amount.map(|amount| amount.normalize().to_string()),
            dose_unit: row.dose_unit,
            frequency: row.frequency,
            start_date: row.start_date,
            end_date: row.end_date,
            notes: row.notes,
            max_daily_doses: row.max_daily_doses,
            min_hours_between_doses: row.min_hours_between_doses.map(|hours| hours.to_string()),
            dose_cycle: row
                .dose_cycle
                .map(|cycle| {
                    crate::models::care::treatment_command::DoseCycle::from_database(cycle)
                        .ok_or(OperationError::Unavailable)
                })
                .transpose()?,
            schedule_type: crate::models::care::treatment_command::ScheduleType::from_database(
                row.schedule_type,
            )
            .ok_or(OperationError::Unavailable)?,
            schedule_config: row.schedule_config,
        })
    }
    pub(crate) fn etag(&self) -> String {
        self.etag.clone()
    }
}

pub(crate) enum PersonalAction {
    Edit(treatments::ValidatedScheduleInput),
    Pause,
    Resume,
    Retire,
}

pub(crate) async fn own_plans(
    db: &DatabaseTransaction,
    actor: AccountId,
) -> Result<Vec<PersonalPlan>, OperationError> {
    let person = access::person_queries::account_person(access::Actor {
        account_id: actor.get(),
    })
    .one(db)
    .await?
    .ok_or(OperationError::Unauthenticated)?;
    person_schedule::Entity::find()
        .filter(person_schedule::Column::PersonId.eq(person.id))
        .order_by_asc(person_schedule::Column::CreatedAt)
        .order_by_asc(person_schedule::Column::Id)
        .all(db)
        .await?
        .into_iter()
        .map(PersonalPlan::from_record)
        .collect()
}

pub(crate) async fn audit_read(
    db: &DatabaseTransaction,
    actor: AccountId,
    request_id: &str,
    provenance: &CredentialProvenance,
) -> Result<(), OperationError> {
    version::ActiveModel {
        item_type: Set("AccountClinicalAccess".into()), item_id: Set(actor.get()), event: Set("personal_plans_read".into()),
        request_id: Set(Some(request_id.into())), created_at: Set(Some(Utc::now().naive_utc())),
        audit_context: Set(json!({"actor_account_id":actor.get(),"household_id":null,"request_id":request_id,"authentication_method":provenance.method.label(),"session_reference":format!("{}:{}",provenance.method.reference_prefix(),provenance.reference),"policy_class":"PersonalClinicalAccess","policy_query":"read_self_schedules"})),
        ..Default::default()
    }.insert(db).await?;
    Ok(())
}

pub(crate) async fn update_personal(
    db: &DatabaseTransaction,
    actor: AccountId,
    id: PersonScheduleId,
    expected: ClinicalRevision,
    action: PersonalAction,
    request_id: &str,
    provenance: &CredentialProvenance,
) -> Result<PersonalPlan, OperationError> {
    lock_current_execution(db, actor, &id).await?;
    let person = access::person_queries::account_person(access::Actor {
        account_id: actor.get(),
    })
    .lock_exclusive()
    .one(db)
    .await?
    .ok_or(OperationError::Unauthenticated)?;
    let row = person_schedule::Entity::find_by_id(id.as_str())
        .filter(person_schedule::Column::PersonId.eq(person.id))
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or(OperationError::NotFound)?;
    if row.clinical_revision != expected.get() {
        return Err(stale());
    }
    if row.retired_at.is_some() {
        return Err(OperationError::NotFound);
    }
    let mut next = row.clone();
    match action {
        PersonalAction::Edit(input) => treatments::amend_personal(&mut next, &input)?,
        PersonalAction::Pause => {
            if next.active {
                next.pause_started_at = Some(Utc::now().naive_utc());
                next.pause_reason = Some("temporarily_not_needed".into());
                next.pause_note = None;
            }
            next.active = false;
        }
        PersonalAction::Resume => {
            next.active = true;
            next.pause_started_at = None;
            next.pause_reason = None;
            next.pause_note = None;
        }
        PersonalAction::Retire => {
            next.active = false;
            next.retired_at = Some(Utc::now().naive_utc());
        }
    }
    if next == row {
        return PersonalPlan::from_record(row);
    }
    next.clinical_revision += 1;
    next.updated_at = Utc::now().naive_utc();
    let active = next.clone().into_active_model().reset_all();
    let saved = active.update(db).await?;
    project_current_execution(db, actor, &saved, request_id).await?;
    version::ActiveModel {
        item_type: Set("PersonSchedule".into()), item_id: Set(saved.origin_schedule_id), event: Set("update".into()),
        object: Set(Some(serde_json::to_string(&row).map_err(|_| OperationError::Unavailable)?)),
        object_changes: Set(Some(serde_json::to_string(&saved).map_err(|_| OperationError::Unavailable)?)),
        request_id: Set(Some(request_id.into())), created_at: Set(Some(Utc::now().naive_utc())),
        audit_context: Set(json!({"actor_account_id":actor.get(),"actor_person_id":person.id,"household_id":null,"request_id":request_id,"person_schedule_id":saved.id,"clinical_revision":saved.clinical_revision,"authentication_method":provenance.method.label(),"session_reference":format!("{}:{}",provenance.method.reference_prefix(),provenance.reference),"policy_class":"PersonalClinicalAccess","policy_query":"update_self_schedule"})),
        ..Default::default()
    }.insert(db).await?;
    PersonalPlan::from_record(saved)
}

fn stale() -> OperationError {
    OperationError::Conflict {
        code: "conflict".into(),
        details: json!({"error":"Record has changed since it was last read"}),
    }
}

async fn context(
    db: &DatabaseTransaction,
    member: Option<&membership::Model>,
) -> Result<(), OperationError> {
    let query = Query::select()
        .expr(
            Func::cust("pg_catalog.set_config").args([
                Expr::val("med_tracker.current_household_id"),
                Expr::val(
                    member
                        .map(|row| row.household_id.to_string())
                        .unwrap_or_default(),
                ),
                Expr::val(true),
            ]),
        )
        .expr(Func::cust("pg_catalog.set_config").args([
            Expr::val("med_tracker.current_membership_id"),
            Expr::val(member.map(|row| row.id.to_string()).unwrap_or_default()),
            Expr::val(true),
        ]))
        .to_owned();
    db.execute_raw(db.get_database_backend().build(&query))
        .await?;
    Ok(())
}

async fn project_current_execution(
    db: &DatabaseTransaction,
    actor: AccountId,
    plan: &person_schedule::Model,
    request_id: &str,
) -> Result<(), OperationError> {
    let member = membership::Entity::find()
        .inner_join(household::Entity)
        .filter(membership::Column::PersonId.eq(plan.person_id))
        .filter(membership::Column::HouseholdId.eq(plan.origin_household_id))
        .filter(membership::Column::Status.eq("active"))
        .filter(membership::Column::RevokedAt.is_null())
        .filter(household::Column::Status.eq("active"))
        .filter(household::Column::LifecycleState.eq("active"))
        .lock_shared()
        .one(db)
        .await?;
    let Some(member) = member else {
        return Ok(());
    };
    context(db, Some(&member)).await?;
    let row = schedule::Entity::find()
        .filter(schedule::Column::PersonScheduleId.eq(&plan.id))
        .filter(schedule::Column::HouseholdId.eq(member.household_id))
        .filter(schedule::Column::ExecutionEndedAt.is_null())
        .lock_exclusive()
        .one(db)
        .await?;
    if let Some(row) = row {
        sync_pause(db, &row, plan, member.id).await?;
        let row_id = row.id;
        let portable_id = row.portable_id.clone();
        let mut execution = row.into_active_model();
        project_clinical(plan, &mut execution);
        let mut options = dosage::Entity::find()
            .filter(dosage::Column::MedicationId.eq(plan.origin_stock_id))
            .filter(dosage::Column::Amount.eq(plan.dose_amount))
            .filter(dosage::Column::Unit.eq(plan.dose_unit.clone()))
            .all(db)
            .await?;
        execution.source_dosage_option_id = Set(if options.len() == 1 {
            options.pop().map(|row| row.id)
        } else {
            None
        });
        execution.update(db).await?;
        let now = Utc::now().naive_utc();
        api_change_event::ActiveModel {
            household_id: Set(member.household_id),
            household_membership_id: Set(Some(member.id)),
            account_id: Set(Some(actor.get())),
            action: Set("update".into()),
            record_type: Set("Schedule".into()),
            record_id: Set(row_id),
            record_portable_id: Set(Some(portable_id.clone())),
            request_id: Set(Some(request_id.into())),
            metadata: Set(
                json!({"record_type":"Schedule","record_id":row_id,"portable_id":portable_id}),
            ),
            occurred_at: Set(now),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    context(db, None).await
}

async fn lock_current_execution(
    db: &DatabaseTransaction,
    actor: AccountId,
    id: &PersonScheduleId,
) -> Result<(), OperationError> {
    let person = access::person_queries::account_person(access::Actor {
        account_id: actor.get(),
    })
    .one(db)
    .await?
    .ok_or(OperationError::Unauthenticated)?;
    let plan = person_schedule::Entity::find_by_id(id.as_str())
        .filter(person_schedule::Column::PersonId.eq(person.id))
        .one(db)
        .await?
        .ok_or(OperationError::NotFound)?;
    let candidate = membership::Entity::find()
        .filter(membership::Column::PersonId.eq(person.id))
        .filter(membership::Column::HouseholdId.eq(plan.origin_household_id))
        .filter(membership::Column::Status.eq("active"))
        .filter(membership::Column::RevokedAt.is_null())
        .one(db)
        .await?;
    if let Some(candidate) = candidate {
        household::Entity::find_by_id(candidate.household_id)
            .lock_exclusive()
            .one(db)
            .await?
            .ok_or(OperationError::NotFound)?;
        let candidate = membership::Entity::find_by_id(candidate.id)
            .filter(membership::Column::Status.eq("active"))
            .filter(membership::Column::RevokedAt.is_null())
            .lock_shared()
            .one(db)
            .await?;
        if let Some(candidate) = candidate {
            context(db, Some(&candidate)).await?;
            schedule::Entity::find()
                .filter(schedule::Column::PersonScheduleId.eq(&plan.id))
                .filter(schedule::Column::HouseholdId.eq(candidate.household_id))
                .filter(schedule::Column::ExecutionEndedAt.is_null())
                .lock_exclusive()
                .one(db)
                .await?;
            context(db, None).await?;
        }
    }
    Ok(())
}

pub(super) async fn sync_pause(
    db: &DatabaseTransaction,
    row: &schedule::Model,
    plan: &person_schedule::Model,
    actor_membership_id: i64,
) -> Result<(), OperationError> {
    if row.active != plan.active {
        let open = pause_period::Entity::find()
            .filter(pause_period::Column::ScheduleId.eq(row.id))
            .filter(pause_period::Column::EndedAt.is_null())
            .lock_exclusive()
            .one(db)
            .await?;
        if plan.active {
            if let Some(open) = open {
                let mut period = open.into_active_model();
                period.ended_at = Set(Some(plan.updated_at));
                period.resumed_by_membership_id = Set(Some(actor_membership_id));
                period.updated_at = Set(plan.updated_at);
                period.update(db).await?;
            }
        } else if plan.retired_at.is_none() && open.is_none() {
            pause_period::ActiveModel {
                household_id: Set(row.household_id),
                portable_id: Set(Uuid::new_v4().to_string()),
                schedule_id: Set(Some(row.id)),
                reason: Set(plan
                    .pause_reason
                    .clone()
                    .unwrap_or_else(|| "temporarily_not_needed".into())),
                note: Set(plan.pause_note.clone()),
                started_at: Set(plan.pause_started_at),
                recorded_by_membership_id: Set(Some(actor_membership_id)),
                legacy_context: Set(false),
                imported_context: Set(false),
                imported_actor_references: Set(json!({})),
                created_at: Set(plan.updated_at),
                updated_at: Set(plan.updated_at),
                ..Default::default()
            }
            .insert(db)
            .await?;
        }
    }
    Ok(())
}
