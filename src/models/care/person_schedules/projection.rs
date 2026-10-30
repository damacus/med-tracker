use super::*;
use crate::models::entities::{api_change_event, shared_schedule, sharing_approval};
use sea_orm::{ConnectionTrait, QueryOrder, TryIntoModel};

pub(crate) async fn publish(
    db: &impl ConnectionTrait,
    plan: &person_schedule::Model,
) -> Result<(), OperationError> {
    let existing = shared_schedule::Entity::find_by_id(&plan.id)
        .lock_exclusive()
        .one(db)
        .await?;
    let mut safe = existing.as_ref().map_or_else(
        || shared_schedule::ActiveModel {
            id: Set(plan.id.clone()),
            person_id: Set(plan.person_id),
            created_at: Set(plan.created_at),
            latest_taken_at: Set(None),
            ..Default::default()
        },
        |row| row.clone().into_active_model(),
    );
    safe.medicine_name = Set(plan.medicine_name.clone());
    safe.dmd_system = Set(plan.dmd_system.clone());
    safe.dmd_code = Set(plan.dmd_code.clone());
    safe.dmd_concept_class = Set(plan.dmd_concept_class.clone());
    safe.stock_unit = Set(plan.stock_unit.clone());
    safe.stock_dose_default = Set(plan.stock_dose_default);
    safe.stock_default_known = Set(plan.stock_default_known);
    safe.pause_started_at = Set(plan.pause_started_at);
    safe.pause_instance_id = Set(plan.pause_instance_id.clone());
    if existing.is_none() {
        safe.last_closed_pause_instance_id = Set(None);
    } else if let Some(previous) = existing
        .as_ref()
        .and_then(|row| row.pause_instance_id.as_ref())
        && plan.pause_instance_id.is_none()
    {
        safe.last_closed_pause_instance_id = Set(Some(previous.clone()));
    }
    safe.origin_time_zone = Set(plan.origin_time_zone.clone());
    safe.clinical_revision = Set(plan.clinical_revision);
    safe.active = Set(plan.active);
    safe.retired_at = Set(plan.retired_at);
    safe.dose_amount = Set(plan.dose_amount);
    safe.dose_unit = Set(plan.dose_unit.clone());
    safe.dose_cycle = Set(plan.dose_cycle);
    safe.frequency = Set(plan.frequency.clone());
    safe.start_date = Set(plan.start_date);
    safe.end_date = Set(plan.end_date);
    safe.max_daily_doses = Set(plan.max_daily_doses);
    safe.min_hours_between_doses = Set(plan.min_hours_between_doses);
    safe.schedule_type = Set(plan.schedule_type);
    safe.schedule_config = Set(plan.schedule_config.clone());
    safe.updated_at = Set(plan.updated_at);
    if existing.is_some() {
        safe.update(db).await?;
    } else {
        safe.insert(db).await?;
    }
    Ok(())
}

pub(crate) fn stock_matches(plan: &shared_schedule::Model, stock: &medication::Model) -> bool {
    use super::super::stock_identity::ExactProductIdentity;
    let identity = ExactProductIdentity::from_parts(
        plan.dmd_system.as_deref(),
        plan.dmd_concept_class.as_deref(),
        plan.dmd_code.as_deref(),
        plan.stock_unit.as_deref(),
    );
    let default = stock
        .dose_amount
        .and_then(|amount| amount.to_string().parse::<sea_orm::prelude::Decimal>().ok());
    identity.is_some()
        && identity == ExactProductIdentity::from_stock(stock)
        && plan.stock_default_known
        && (stock.dose_amount.is_none() || default.is_some())
        && default == plan.stock_dose_default
}

pub(crate) fn project(plan: &shared_schedule::Model, row: &mut schedule::ActiveModel) {
    row.active = Set(plan.active);
    row.retired_at = Set(plan.retired_at);
    row.dose_amount = Set(plan.dose_amount);
    row.dose_unit = Set(plan.dose_unit.clone());
    row.dose_cycle = Set(plan.dose_cycle);
    row.frequency = Set(plan.frequency.clone());
    row.start_date = Set(plan.start_date);
    row.end_date = Set(plan.end_date);
    row.notes = Set(None);
    row.source_dosage_option_id = Set(None);
    row.max_daily_doses = Set(plan.max_daily_doses);
    row.min_hours_between_doses = Set(plan.min_hours_between_doses);
    row.schedule_type = Set(plan.schedule_type);
    row.schedule_config = Set(plan.schedule_config.clone());
    row.origin_time_zone = Set(Some(plan.origin_time_zone.clone()));
    row.clinical_revision = Set(Some(plan.clinical_revision));
    row.updated_at = Set(plan.updated_at);
}

pub(crate) async fn refresh(tenant: &TenantTransaction) -> Result<(), OperationError> {
    refresh_for(tenant, None).await
}

pub(super) async fn refresh_for(
    tenant: &TenantTransaction,
    person: Option<i64>,
) -> Result<(), OperationError> {
    super::super::sync::lock(tenant).await?;
    let rows = schedule::Entity::find()
        .filter(schedule::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(schedule::Column::PersonScheduleId.is_not_null())
        .filter(schedule::Column::ExecutionEndedAt.is_null());
    let rows = if let Some(person) = person {
        rows.filter(schedule::Column::PersonId.eq(person))
    } else {
        rows
    };
    let rows = rows
        .order_by_asc(schedule::Column::PersonScheduleId)
        .all(tenant.transaction())
        .await?;
    for row in rows {
        refresh_one(tenant, row.id).await?;
    }
    Ok(())
}

/// Read-side projection for paths that must not take the write locks
/// `refresh_one` holds (household sync lock, shared state and source row
/// FOR UPDATE). It applies the same canonical merge in memory — no
/// `sync::lock`, no row locks, no persistence — so ordinary reads never
/// wait on an exclusive source write lock. Denials stay fail-closed: a
/// missing safe state or an ended approval still rejects, and a canonically
/// retired source disappears exactly as a refreshed row would.
pub(crate) async fn refresh_read(
    tenant: &TenantTransaction,
    initial: schedule::Model,
) -> Result<schedule::Model, OperationError> {
    let db = tenant.transaction();
    let Some(plan) = initial.person_schedule_id.as_ref() else {
        return Ok(initial);
    };
    let state = shared_schedule::Entity::find_by_id(plan)
        .one(db)
        .await?
        .ok_or(OperationError::NotFound)?;
    let link = crate::models::entities::shared_execution::Entity::find_by_id(initial.id)
        .one(db)
        .await?;
    let approval_id = initial
        .sharing_approval_id
        .or(link.as_ref().and_then(|link| link.approval_id));
    if let Some(approval) = approval_id {
        sharing_approval::Entity::find_by_id(approval)
            .filter(sharing_approval::Column::EndedAt.is_null())
            .one(db)
            .await?
            .ok_or(OperationError::Forbidden)?;
    }
    let mut next = initial.clone().into_active_model();
    if initial.canonical_origin {
        let canonical = person_schedule::Entity::find_by_id(plan)
            .one(db)
            .await?
            .ok_or(OperationError::NotFound)?;
        if canonical.clinical_revision != state.clinical_revision {
            return Err(OperationError::Unavailable);
        }
        super::project_clinical(&canonical, &mut next);
    } else {
        project(&state, &mut next);
    }
    let merged = next
        .try_into_model()
        .map_err(|_| OperationError::Unavailable)?;
    if merged.retired_at.is_some() || merged.execution_ended_at.is_some() {
        return Err(OperationError::NotFound);
    }
    Ok(merged)
}

pub(crate) async fn refresh_one(
    tenant: &TenantTransaction,
    id: i64,
) -> Result<schedule::Model, OperationError> {
    // Every execution writer/projection takes household, safe state, then source.
    // Full current-state reads visit canonical UUIDs in order; writers refresh only
    // their selected source, without acquiring unrelated states after a lock wait.
    super::super::sync::lock(tenant).await?;
    let db = tenant.transaction();
    let initial = schedule::Entity::find_by_id(id)
        .filter(schedule::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(schedule::Column::ExecutionEndedAt.is_null())
        .one(db)
        .await?
        .ok_or(OperationError::NotFound)?;
    let Some(plan) = initial.person_schedule_id.as_ref() else {
        return schedule::Entity::find_by_id(id)
            .filter(schedule::Column::HouseholdId.eq(tenant.scope().household_id))
            .filter(schedule::Column::ExecutionEndedAt.is_null())
            .lock_exclusive()
            .one(db)
            .await?
            .ok_or(OperationError::NotFound);
    };
    shared_schedule::Entity::find_by_id(plan)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or(OperationError::NotFound)?;
    let link = crate::models::entities::shared_execution::Entity::find_by_id(id)
        .one(db)
        .await?;
    let approval_id = initial
        .sharing_approval_id
        .or(link.as_ref().and_then(|link| link.approval_id));
    if let Some(approval) = approval_id {
        sharing_approval::Entity::find_by_id(approval)
            .filter(sharing_approval::Column::EndedAt.is_null())
            .lock_shared()
            .one(db)
            .await?
            .ok_or(OperationError::Forbidden)?;
    }
    let row = schedule::Entity::find_by_id(id)
        .filter(schedule::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(schedule::Column::ExecutionEndedAt.is_null())
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or(OperationError::NotFound)?;
    crate::models::access::recheck(tenant).await?;
    let state = shared_schedule::Entity::find_by_id(plan)
        .one(db)
        .await?
        .ok_or(OperationError::Forbidden)?;
    if row.person_schedule_id.as_ref() != Some(plan)
        || row.sharing_approval_id != initial.sharing_approval_id
        || crate::models::entities::shared_execution::Entity::find_by_id(id)
            .one(db)
            .await?
            .as_ref()
            .and_then(|link| link.approval_id)
            != link.as_ref().and_then(|link| link.approval_id)
    {
        return Err(OperationError::Forbidden);
    }
    super::pauses::project_owned(db, &row).await?;
    if row.clinical_revision == Some(state.clinical_revision) {
        return Ok(row);
    }
    let before = serde_json::json!({"person_schedule_id":plan,"clinical_revision":row.clinical_revision,
        "active":row.active,"retired_at":row.retired_at});
    let origin = row.canonical_origin;
    let mut next = row.into_active_model();
    if origin {
        let canonical = person_schedule::Entity::find_by_id(plan)
            .one(db)
            .await?
            .ok_or(OperationError::NotFound)?;
        if canonical.clinical_revision != state.clinical_revision {
            return Err(OperationError::Unavailable);
        }
        super::project_clinical(&canonical, &mut next);
    } else {
        project(&state, &mut next);
    }
    let next = next.update(db).await?;
    super::super::administration::persistence::record_version_as(
        tenant,
        "SharedScheduleProjection",
        next.id,
        "project",
        Some(before),
        serde_json::json!({"person_schedule_id":plan,"clinical_revision":state.clinical_revision,
            "active":state.active,"retired_at":state.retired_at}),
        None,
    )
    .await?;
    let now = Utc::now().naive_utc();
    api_change_event::ActiveModel {
        household_id: Set(next.household_id),
        household_membership_id: Set(Some(tenant.membership().id)),
        account_id: Set(Some(tenant.scope().actor.account_id)),
        action: Set("update".into()),
        record_type: Set("Schedule".into()),
        record_id: Set(next.id),
        record_portable_id: Set(Some(next.portable_id.clone())),
        request_id: Set(Some(tenant.scope().request_id.clone())),
        metadata: Set(serde_json::json!({"record_type":"Schedule","record_id":next.id,
            "portable_id":next.portable_id,"projection":true,"clinical_revision":state.clinical_revision})),
        occurred_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(next)
}

pub(crate) async fn list(
    tenant: &TenantTransaction,
) -> Result<Vec<shared_schedule::Model>, OperationError> {
    crate::models::access::recheck(tenant).await?;
    let people = sharing_approval::Entity::find()
        .select_only()
        .column(sharing_approval::Column::PersonId)
        .filter(sharing_approval::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(sharing_approval::Column::EndedAt.is_null());
    use sea_orm::QueryTrait;
    shared_schedule::Entity::find()
        .filter(shared_schedule::Column::PersonId.in_subquery(people.into_query()))
        .filter(shared_schedule::Column::RetiredAt.is_null())
        .order_by_asc(shared_schedule::Column::Id)
        .all(tenant.transaction())
        .await
        .map_err(Into::into)
}

pub(crate) struct BindStock {
    pub(crate) plan: PersonScheduleId,
    pub(crate) revision: ClinicalRevision,
    pub(crate) medication: crate::models::ids::MedicationId,
}

pub(crate) async fn bind(
    tenant: &TenantTransaction,
    command: BindStock,
    provenance: &super::super::doses::CredentialProvenance,
) -> Result<schedule::Model, OperationError> {
    super::super::sync::lock(tenant).await?;
    super::super::administration::authorize(tenant).await?;
    let db = tenant.transaction();
    let state = shared_schedule::Entity::find_by_id(command.plan.as_str())
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or(OperationError::NotFound)?;
    crate::models::access::recheck(tenant).await?;
    let state = shared_schedule::Entity::find_by_id(&state.id)
        .one(db)
        .await?
        .ok_or(OperationError::Forbidden)?;
    if state.clinical_revision != command.revision.get() {
        return Err(OperationError::Conflict {
            code: "conflict".into(),
            details: serde_json::json!({"error":"Plan changed. Review the current prescription before binding stock."}),
        });
    }
    let approval = sharing_approval::Entity::find()
        .filter(sharing_approval::Column::PersonId.eq(state.person_id))
        .filter(sharing_approval::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(sharing_approval::Column::EndedAt.is_null())
        .lock_shared()
        .one(db)
        .await?
        .ok_or(OperationError::Forbidden)?;
    let stock = medication::Entity::find_by_id(command.medication.get())
        .filter(medication::Column::HouseholdId.eq(tenant.scope().household_id))
        .lock_shared()
        .one(db)
        .await?
        .ok_or(OperationError::NotFound)?;
    if !stock_matches(&state, &stock) {
        return Err(super::super::guardian_declarations::validation(
            "medication_id",
            "Stock must match the exact prescribed product and unit",
        ));
    }
    let current = schedule::Entity::find()
        .filter(schedule::Column::PersonScheduleId.eq(&state.id))
        .filter(schedule::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(schedule::Column::ExecutionEndedAt.is_null())
        .lock_exclusive()
        .one(db)
        .await?;
    super::super::administration::authorize(tenant).await?;
    if let Some(row) = current.as_ref() {
        let link = crate::models::entities::shared_execution::Entity::find_by_id(row.id)
            .one(db)
            .await?;
        let effective_approval = row
            .sharing_approval_id
            .or(link.as_ref().and_then(|link| link.approval_id));
        if row.person_id != state.person_id || effective_approval != Some(approval.id) {
            return Err(OperationError::Forbidden);
        }
        record_execution(db, row, Some(approval.id)).await?;
    }
    let before = current
        .as_ref()
        .map(|row| super::super::treatments::representation(row).0["data"].clone());
    let mut row = current.as_ref().map_or_else(
        || schedule::ActiveModel {
            household_id: Set(tenant.scope().household_id),
            person_id: Set(state.person_id),
            person_schedule_id: Set(Some(state.id.clone())),
            sharing_approval_id: Set(Some(approval.id)),
            portable_id: Set(Uuid::new_v4().to_string()),
            execution_ended_at: Set(None),
            created_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        },
        |row| row.clone().into_active_model(),
    );
    row.medication_id = Set(stock.id);
    project(&state, &mut row);
    if let Some(origin) = current.as_ref().filter(|row| row.canonical_origin) {
        row.notes = Set(origin.notes.clone());
    }
    let row = if current.is_some() {
        row.update(db).await?
    } else {
        row.insert(db).await?
    };
    record_execution(db, &row, Some(approval.id)).await?;
    super::super::treatments::persist(
        tenant,
        &row,
        before,
        if current.is_some() {
            "update"
        } else {
            "create"
        },
        Some(provenance),
    )
    .await?;
    super::super::administration::persistence::record_version_as(tenant,"SharedStockBinding",row.id,"bind",None,serde_json::json!({"person_schedule_id":state.id,"clinical_revision":state.clinical_revision,"approval_id":approval.id,"medication_id":stock.id,"stock_unit":state.stock_unit}),Some(provenance)).await?;
    Ok(row)
}

pub(crate) async fn record_execution(
    db: &impl ConnectionTrait,
    row: &schedule::Model,
    approval: Option<i64>,
) -> Result<(), OperationError> {
    use crate::models::entities::shared_execution;
    let expected = shared_execution::Model {
        schedule_id: row.id,
        portable_id: row.portable_id.clone(),
        household_id: row.household_id,
        approval_id: approval,
        person_id: row.person_id,
        person_schedule_id: row
            .person_schedule_id
            .clone()
            .ok_or(OperationError::Unavailable)?,
    };
    if let Some(existing) = shared_execution::Entity::find_by_id(row.id).one(db).await? {
        if existing != expected {
            return Err(OperationError::Forbidden);
        }
    } else {
        expected.into_active_model().reset_all().insert(db).await?;
    }
    Ok(())
}
