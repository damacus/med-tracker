use crate::models::{
    access::TenantTransaction,
    entities::{medication, person_schedule, schedule},
    errors::OperationError,
};
use chrono::Utc;
use chrono_tz::Tz;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, IntoActiveModel, QueryFilter,
    QuerySelect, Set,
};
use uuid::Uuid;
mod adoption_history;
pub(crate) mod legacy;
mod pauses;
mod personal;
mod view;
pub(crate) use view::SharedPlan;
pub(crate) mod projection;
pub(crate) mod shared_commands;
pub(crate) mod sharing;
pub(crate) use personal::audit_read;
pub(crate) use personal::{PersonalAction, PersonalPlan, own_plans, update_personal};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PersonScheduleId(String);

impl PersonScheduleId {
    pub(crate) fn parse(value: &str) -> Result<Self, OperationError> {
        Uuid::parse_str(value).map_err(|_| OperationError::NotFound)?;
        Ok(Self(value.into()))
    }
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ClinicalRevision(i64);

impl ClinicalRevision {
    pub(crate) const fn get(self) -> i64 {
        self.0
    }
    pub(crate) fn parse(value: i64) -> Result<Self, OperationError> {
        (value > 0)
            .then_some(Self(value))
            .ok_or(OperationError::NotFound)
    }
    pub(crate) fn from_etag(id: &PersonScheduleId, value: &str) -> Result<Self, OperationError> {
        let revision = value
            .strip_prefix(&format!("\"person-schedule:{}:", id.as_str()))
            .and_then(|value| value.strip_suffix('"'))
            .and_then(|value| value.parse().ok())
            .ok_or_else(|| crate::models::profile::validation("if_match", "is invalid"))?;
        Self::parse(revision)
    }
}

pub(crate) struct PlanIdentity {
    pub(crate) id: PersonScheduleId,
    pub(crate) revision: ClinicalRevision,
    pub(crate) person_id: crate::models::ids::PersonId,
    pub(crate) zone: Tz,
}

pub(crate) fn identity(row: &person_schedule::Model) -> Result<PlanIdentity, OperationError> {
    Ok(PlanIdentity {
        id: PersonScheduleId::parse(&row.id)?,
        revision: ClinicalRevision(row.clinical_revision),
        person_id: crate::models::ids::PersonId::from_database(row.person_id),
        zone: row
            .origin_time_zone
            .parse()
            .map_err(|_| OperationError::Unavailable)?,
    })
}

pub(crate) fn execution_zone(row: &schedule::Model, fallback: Tz) -> Tz {
    row.origin_time_zone
        .as_deref()
        .and_then(|zone| zone.parse().ok())
        .unwrap_or(fallback)
}

pub(crate) fn current_date_condition(fallback: Tz) -> sea_orm::Condition {
    use sea_orm::sea_query::{Alias, Expr, ExprTrait, Func};
    let zone = Func::coalesce([
        Expr::col(schedule::Column::OriginTimeZone),
        Expr::val(fallback.name()),
    ]);
    let local_date =
        Expr::expr(Func::cust("timezone").args([Expr::expr(zone), Expr::val(Utc::now())]))
            .cast_as(Alias::new("date"));
    sea_orm::Condition::all()
        .add(Expr::col(schedule::Column::StartDate).lte(local_date.clone()))
        .add(Expr::col(schedule::Column::EndDate).gte(local_date))
}

pub(crate) async fn save_execution(
    tenant: &TenantTransaction,
    row: schedule::Model,
    new_zone: Option<Tz>,
    configured_zone: Tz,
    provenance: Option<&super::doses::CredentialProvenance>,
) -> Result<schedule::Model, OperationError> {
    let db = tenant.transaction();
    if row.execution_ended_at.is_some() {
        return Err(OperationError::NotFound);
    }
    let current = if let Some(id) = row.person_schedule_id.as_ref() {
        crate::models::entities::shared_schedule::Entity::find_by_id(id)
            .lock_exclusive()
            .one(db)
            .await?
            .ok_or(OperationError::NotFound)?;
        crate::models::access::recheck(tenant).await?;
        Some(
            person_schedule::Entity::find_by_id(id)
                .lock_exclusive()
                .one(db)
                .await?
                .ok_or(OperationError::NotFound)?,
        )
    } else {
        None
    };
    let previous = current.clone();
    let shared_before = if row.sharing_approval_id.is_some() {
        Some(
            crate::models::entities::shared_schedule::Entity::find_by_id(
                row.person_schedule_id
                    .as_ref()
                    .ok_or(OperationError::Unavailable)?,
            )
            .one(db)
            .await?
            .ok_or(OperationError::NotFound)?,
        )
    } else {
        None
    };
    let mut plan = if let Some(current) = current {
        current.into_active_model()
    } else {
        person_schedule::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            person_id: Set(row.person_id),
            origin_household_id: Set(row.household_id),
            origin_schedule_id: Set(row.id),
            origin_time_zone: Set(new_zone.unwrap_or(configured_zone).name().into()),
            timezone_provenance: Set(if new_zone.is_some() {
                "captured_sharer_zone"
            } else {
                "adopted_application_zone"
            }
            .into()),
            clinical_revision: Set(0),
            created_at: Set(row.created_at),
            ..Default::default()
        }
    };
    if plan.origin_stock_id.is_not_set()
        || (row.sharing_approval_id.is_none()
            && plan.origin_stock_id.clone().unwrap() != row.medication_id)
    {
        let stock = medication::Entity::find_by_id(row.medication_id)
            .one(db)
            .await?
            .ok_or(OperationError::NotFound)?;
        let default = stock
            .dose_amount
            .and_then(|amount| amount.to_string().parse::<sea_orm::prelude::Decimal>().ok());
        plan.stock_default_known = Set(stock.dose_amount.is_none() || default.is_some());
        plan.stock_dose_default = Set(default);
        plan.origin_stock_id = Set(stock.id);
        plan.origin_stock_portable_id = Set(stock.portable_id);
        plan.medicine_name = Set(stock.name);
        plan.dmd_system = Set(stock.dmd_system);
        plan.dmd_code = Set(stock.dmd_code);
        plan.dmd_concept_class = Set(stock.dmd_concept_class);
        plan.stock_unit = Set(stock.dose_unit);
    }
    plan.active = Set(row.active);
    plan.retired_at = Set(row.retired_at);
    plan.dose_amount = Set(row.dose_amount);
    plan.dose_unit = Set(row.dose_unit.clone());
    plan.dose_cycle = Set(row.dose_cycle);
    plan.frequency = Set(row.frequency.clone());
    plan.start_date = Set(row.start_date);
    plan.end_date = Set(row.end_date);
    if row.sharing_approval_id.is_none() {
        plan.notes = Set(row.notes.clone());
    }
    plan.max_daily_doses = Set(row.max_daily_doses);
    plan.min_hours_between_doses = Set(row.min_hours_between_doses);
    plan.schedule_type = Set(row.schedule_type);
    plan.schedule_config = Set(row.schedule_config.clone());
    if row.active && previous.as_ref().is_some_and(|previous| !previous.active) {
        plan.pause_started_at = Set(None);
        plan.pause_instance_id = Set(None);
        plan.pause_reason = Set(None);
        plan.pause_note = Set(None);
    } else if !row.active {
        let pause = crate::models::entities::pause_period::Entity::find()
            .filter(crate::models::entities::pause_period::Column::ScheduleId.eq(row.id))
            .filter(crate::models::entities::pause_period::Column::EndedAt.is_null())
            .one(db)
            .await?;
        if let Some(pause) = pause
            && previous.as_ref().is_none_or(|previous| {
                previous.active || previous.pause_instance_id.as_ref() == Some(&pause.portable_id)
            })
        {
            plan.pause_started_at = Set(pause.started_at);
            plan.pause_instance_id = Set(Some(pause.portable_id));
            plan.pause_reason = Set(if row.sharing_approval_id.is_none() {
                Some(pause.reason)
            } else {
                None
            });
            plan.pause_note = Set(if row.sharing_approval_id.is_none() {
                pause.note
            } else {
                None
            });
        }
    }
    if let Some(zone) = row.origin_time_zone.as_ref() {
        if plan.origin_time_zone.clone().unwrap() != *zone {
            plan.origin_time_zone = Set(zone.clone());
            plan.timezone_provenance = Set("authorised_correction".into());
        }
    }
    plan.clinical_revision = Set(plan.clinical_revision.clone().unwrap() + 1);
    let resumed_at = if row.active
        && let Some(instance) = previous
            .as_ref()
            .and_then(|previous| previous.pause_instance_id.as_ref())
    {
        crate::models::entities::pause_period::Entity::find()
            .select_only()
            .column(crate::models::entities::pause_period::Column::EndedAt)
            .filter(crate::models::entities::pause_period::Column::ScheduleId.eq(row.id))
            .filter(crate::models::entities::pause_period::Column::PortableId.eq(instance))
            .into_tuple::<Option<chrono::NaiveDateTime>>()
            .one(db)
            .await?
            .flatten()
    } else {
        None
    };
    plan.updated_at = Set(resumed_at.unwrap_or_else(|| Utc::now().naive_utc()));
    let plan = if row.person_schedule_id.is_some() {
        plan.update(db).await?
    } else {
        plan.insert(db).await?
    };
    projection::publish(db, &plan).await?;
    if let Some(before) = shared_before {
        let after = crate::models::entities::shared_schedule::Entity::find_by_id(&plan.id)
            .one(db)
            .await?
            .ok_or(OperationError::NotFound)?;
        let declaration = if tenant.membership().person_id == plan.person_id {
            None
        } else {
            Some(
                sharing::guardian(
                    db,
                    crate::models::ids::PersonId::from_database(plan.person_id),
                )
                .await?
                .ok_or(OperationError::Forbidden)?,
            )
        };
        let mut context = crate::models::audit_context::household(tenant, provenance);
        context["person_schedule_id"] = serde_json::json!(plan.id);
        context["subject_person_id"] = serde_json::json!(plan.person_id);
        context["clinical_revision"] = serde_json::json!(plan.clinical_revision);
        context["guardian_declaration_id"] = serde_json::json!(declaration);
        context["policy_class"] = serde_json::json!("SharedPersonScheduleClinical");
        crate::models::entities::version::Entity::insert(
            crate::models::entities::version::ActiveModel {
                item_type: Set("SharedPersonSchedule".into()),
                item_id: Set(plan.person_id),
                event: Set("update".into()),
                object_changes: Set(Some(view::clinical_changes(&before, &after)?.to_string())),
                whodunnit: Set(Some(tenant.scope().actor.account_id.to_string())),
                household_id: Set(Some(tenant.scope().household_id)),
                actor_membership_id: Set(Some(tenant.membership().id)),
                request_id: Set(Some(tenant.scope().request_id.clone())),
                created_at: Set(Some(plan.updated_at)),
                audit_context: Set(context),
                ..Default::default()
            },
        )
        .exec_without_returning(db)
        .await?;
    } else {
        super::administration::persistence::record_version_as(
            tenant,
            "PersonSchedule",
            plan.origin_schedule_id,
            if previous.is_some() {
                "update"
            } else {
                "create"
            },
            previous
                .as_ref()
                .map(|previous| serde_json::json!(previous)),
            serde_json::json!(&plan),
            provenance,
        )
        .await?;
    }
    pauses::record_resume(
        db,
        previous
            .as_ref()
            .and_then(|row| row.pause_instance_id.as_deref()),
        &plan.id,
        pauses::ResumeActor {
            account: crate::models::ids::AccountId::from_database(tenant.scope().actor.account_id),
            membership: Some(tenant.membership().id),
            request_id: &tenant.scope().request_id,
            provenance,
        },
    )
    .await?;
    let history = adoption_history::capture(tenant, &row, &plan, provenance).await?;
    let mut linked = row.into_active_model();
    linked.canonical_origin = Set(plan.origin_schedule_id == linked.id.clone().unwrap());
    linked.person_schedule_id = Set(Some(plan.id));
    linked.clinical_revision = Set(Some(plan.clinical_revision));
    linked.origin_time_zone = Set(Some(plan.origin_time_zone));
    let linked = linked.update(db).await?;
    pauses::associate_start(db, &linked).await?;
    use crate::models::entities::sharing_approval;
    let approval = sharing_approval::Entity::find()
        .filter(sharing_approval::Column::HouseholdId.eq(linked.household_id))
        .filter(sharing_approval::Column::PersonId.eq(linked.person_id))
        .filter(sharing_approval::Column::EndedAt.is_null())
        .lock_shared()
        .one(db)
        .await?;
    crate::models::access::recheck(tenant).await?;
    if let Some(approval) = approval {
        projection::record_execution(db, &linked, Some(approval.id)).await?;
    } else if sharing_approval::Entity::find()
        .filter(sharing_approval::Column::HouseholdId.eq(linked.household_id))
        .filter(sharing_approval::Column::PersonId.eq(linked.person_id))
        .one(db)
        .await?
        .is_some()
    {
        return Err(OperationError::Forbidden);
    } else if linked.canonical_origin {
        projection::record_execution(db, &linked, None).await?;
    }
    history
        .seed(
            db,
            linked
                .person_schedule_id
                .as_deref()
                .ok_or(OperationError::Unavailable)?,
        )
        .await?;
    Ok(linked)
}

pub(crate) async fn end_participation(
    tenant: &TenantTransaction,
    person_id: i64,
) -> Result<(), OperationError> {
    let rows = schedule::Entity::find()
        .filter(schedule::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(schedule::Column::PersonId.eq(person_id))
        .filter(schedule::Column::ExecutionEndedAt.is_null())
        .lock_exclusive()
        .all(tenant.transaction())
        .await?;
    let retiring = sharing::clinical_authority(
        tenant.transaction(),
        crate::models::ids::PersonId::from_database(person_id),
    )
    .await?;
    for row in rows {
        let now = Utc::now().naive_utc();
        if row.person_schedule_id.is_none() {
            legacy::freeze(tenant, &row, now).await?;
        }
        let before = super::treatments::representation(&row).0["data"].clone();
        let retired = row.retired_at;
        let mut ended = row.into_active_model();
        if retiring {
            ended.retired_at = Set(retired.or(Some(now)));
            ended.updated_at = Set(now);
            let ended = ended.update(tenant.transaction()).await?;
            let mut closed = ended.into_active_model();
            closed.execution_ended_at = Set(Some(now));
            closed.active = Set(false);
            closed.updated_at = Set(now);
            let ended = closed.update(tenant.transaction()).await?;
            super::treatments::persist(tenant, &ended, Some(before), "update", None).await?;
            continue;
        }
        ended.execution_ended_at = Set(Some(now));
        ended.active = Set(false);
        ended.updated_at = Set(now);
        let ended = ended.update(tenant.transaction()).await?;
        super::treatments::persist(tenant, &ended, Some(before), "update", None).await?;
    }
    sharing::end(
        tenant,
        crate::models::ids::PersonId::from_database(person_id),
    )
    .await?;
    Ok(())
}

pub(crate) fn project_clinical(
    plan: &person_schedule::Model,
    execution: &mut schedule::ActiveModel,
) {
    if execution.dose_amount.clone().unwrap() != plan.dose_amount
        || execution.dose_unit.clone().unwrap() != plan.dose_unit
    {
        execution.source_dosage_option_id = Set(None);
    }
    execution.active = Set(plan.active);
    execution.retired_at = Set(plan.retired_at);
    execution.dose_amount = Set(plan.dose_amount);
    execution.dose_unit = Set(plan.dose_unit.clone());
    execution.dose_cycle = Set(plan.dose_cycle);
    execution.frequency = Set(plan.frequency.clone());
    execution.start_date = Set(plan.start_date);
    execution.end_date = Set(plan.end_date);
    execution.notes = Set(plan.notes.clone());
    execution.max_daily_doses = Set(plan.max_daily_doses);
    execution.min_hours_between_doses = Set(plan.min_hours_between_doses);
    execution.schedule_type = Set(plan.schedule_type);
    execution.schedule_config = Set(plan.schedule_config.clone());
    execution.origin_time_zone = Set(Some(plan.origin_time_zone.clone()));
    execution.clinical_revision = Set(Some(plan.clinical_revision));
    execution.updated_at = Set(plan.updated_at);
}

pub(crate) async fn restore_participation(
    tenant: &TenantTransaction,
    person_id: i64,
) -> Result<(), OperationError> {
    projection::refresh_for(tenant, Some(person_id)).await
}

pub(crate) fn stock_matches(plan: &person_schedule::Model, stock: &medication::Model) -> bool {
    use super::stock_identity::ExactProductIdentity;
    let prescribed = ExactProductIdentity::from_parts(
        plan.dmd_system.as_deref(),
        plan.dmd_concept_class.as_deref(),
        plan.dmd_code.as_deref(),
        plan.stock_unit.as_deref(),
    );
    if let (Some(prescribed), Some(candidate)) =
        (prescribed, ExactProductIdentity::from_stock(stock))
    {
        let same_row = stock.household_id == plan.origin_household_id
            && stock.id == plan.origin_stock_id
            && stock.portable_id == plan.origin_stock_portable_id;
        let default = stock
            .dose_amount
            .and_then(|amount| amount.to_string().parse::<sea_orm::prelude::Decimal>().ok());
        return prescribed == candidate
            && (same_row
                || (plan.stock_default_known
                    && (stock.dose_amount.is_none() || default.is_some())
                    && default == plan.stock_dose_default));
    }
    stock.household_id == plan.origin_household_id
        && stock.id == plan.origin_stock_id
        && stock.portable_id == plan.origin_stock_portable_id
        && stock.dmd_system == plan.dmd_system
        && stock.dmd_concept_class == plan.dmd_concept_class
        && stock.dmd_code == plan.dmd_code
        && stock.dose_unit == plan.stock_unit
}
