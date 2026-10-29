use super::*;
use crate::models::{
    care::doses::CredentialProvenance,
    entities::{account, legacy_prescription, pause_period, shared_schedule, version},
    identity::resource::ValidatedPrincipal,
    ids::{AccountId, PersonId},
};
use sea_orm::{
    ConnectionTrait, QueryOrder,
    sea_query::{Expr, Func, Query},
};

pub(super) async fn freeze(
    tenant: &TenantTransaction,
    row: &schedule::Model,
    ended_at: chrono::NaiveDateTime,
) -> Result<(), OperationError> {
    let db = tenant.transaction();
    let stock = medication::Entity::find_by_id(row.medication_id)
        .filter(medication::Column::HouseholdId.eq(tenant.scope().household_id))
        .lock_shared()
        .one(db)
        .await?
        .ok_or(OperationError::NotFound)?;
    let pauses = pause_period::Entity::find()
        .select_only()
        .columns([
            pause_period::Column::PortableId,
            pause_period::Column::StartedAt,
        ])
        .filter(pause_period::Column::ScheduleId.eq(row.id))
        .filter(pause_period::Column::HouseholdId.eq(row.household_id))
        .filter(pause_period::Column::EndedAt.is_null())
        .order_by_asc(pause_period::Column::Id)
        .lock_shared()
        .into_tuple::<(String, Option<chrono::NaiveDateTime>)>()
        .all(db)
        .await?;
    if pauses.len() > 1 {
        return Err(OperationError::Unavailable);
    }
    crate::models::care::administration::authorize(tenant).await?;
    let pause = (!row.active).then(|| pauses.first()).flatten();
    let stock_default = stock
        .dose_amount
        .and_then(|amount| amount.to_string().parse().ok());
    legacy_prescription::Entity::insert(legacy_prescription::ActiveModel {
        schedule_id: Set(row.id),
        source_portable_id: Set(row.portable_id.clone()),
        person_id: Set(row.person_id),
        household_id: Set(row.household_id),
        origin_stock_id: Set(stock.id),
        origin_stock_portable_id: Set(stock.portable_id),
        medicine_name: Set(stock.name),
        dmd_system: Set(stock.dmd_system),
        dmd_code: Set(stock.dmd_code),
        dmd_concept_class: Set(stock.dmd_concept_class),
        stock_unit: Set(stock.dose_unit),
        stock_dose_default: Set(stock_default),
        stock_default_known: Set(stock.dose_amount.is_none() || stock_default.is_some()),
        active: Set(row.active),
        retired_at: Set(row.retired_at),
        dose_amount: Set(row.dose_amount),
        dose_unit: Set(row.dose_unit.clone()),
        dose_cycle: Set(row.dose_cycle),
        frequency: Set(row.frequency.clone()),
        start_date: Set(row.start_date),
        end_date: Set(row.end_date),
        max_daily_doses: Set(row.max_daily_doses),
        min_hours_between_doses: Set(row.min_hours_between_doses),
        schedule_type: Set(row.schedule_type),
        schedule_config: Set(row.schedule_config.clone()),
        origin_time_zone: Set(row.origin_time_zone.clone()),
        pause_started_at: Set(pause.and_then(|(_, started)| *started)),
        pause_instance_id: Set(pause.map(|(instance, _)| instance.clone())),
        source_created_at: Set(row.created_at),
        frozen_at: Set(ended_at),
        source_ended_at: Set(ended_at),
        source_closed_active: Set(false),
        captured_by_account_id: Set(tenant.scope().actor.account_id),
        captured_by_membership_id: Set(tenant.membership().id),
        request_id: Set(tenant.scope().request_id.clone()),
        adopted_person_schedule_id: Set(None),
    })
    .exec_without_returning(db)
    .await?;
    Ok(())
}

struct AdoptionActor {
    account: AccountId,
    person: PersonId,
    subject: PersonId,
    authority: sharing::ConsentAuthority,
}

impl AdoptionActor {
    async fn capture(
        db: &DatabaseTransaction,
        principal: &ValidatedPrincipal,
        subject: i64,
    ) -> Result<Self, super::shared_commands::Failure> {
        let actual = account::Entity::find_by_id(principal.account_id())
            .one(db)
            .await?
            .ok_or(OperationError::Unauthenticated)?;
        let subject = PersonId::from_database(subject);
        let authority = if actual.person_id == subject.get() {
            sharing::ConsentAuthority::SelfPerson
        } else {
            sharing::ConsentAuthority::Guardian(
                crate::models::care::guardian_declarations::DeclarationId::from_database(
                    sharing::guardian(db, subject)
                        .await?
                        .ok_or(OperationError::Forbidden)?,
                ),
            )
        };
        Ok(Self {
            account: AccountId::from_database(actual.id),
            person: PersonId::from_database(actual.person_id),
            subject,
            authority,
        })
    }
    async fn current(
        &self,
        db: &DatabaseTransaction,
        principal: &ValidatedPrincipal,
        settings: Option<&serde_json::Value>,
    ) -> Result<(), super::shared_commands::Failure> {
        principal.revalidate_account(db, settings).await?;
        let actual = account::Entity::find_by_id(self.account.get())
            .one(db)
            .await?
            .ok_or(OperationError::Unauthenticated)?;
        if actual.person_id != self.person.get()
            || (matches!(self.authority, sharing::ConsentAuthority::SelfPerson)
                && self.person != self.subject)
            || !self.authority.current(db, self.subject).await?
        {
            return Err(OperationError::Forbidden.into());
        }
        Ok(())
    }
    fn declaration(&self) -> Option<i64> {
        match self.authority {
            sharing::ConsentAuthority::SelfPerson => None,
            sharing::ConsentAuthority::Guardian(id) => Some(id.get()),
        }
    }
}

pub(crate) async fn adopt(
    db: &DatabaseTransaction,
    principal: &ValidatedPrincipal,
    portable_id: &str,
    request_id: &str,
    settings: Option<&serde_json::Value>,
) -> Result<SharedPlan, super::shared_commands::Failure> {
    Uuid::parse_str(portable_id).map_err(|_| OperationError::NotFound)?;
    let visible = legacy_prescription::Entity::find()
        .filter(legacy_prescription::Column::SourcePortableId.eq(portable_id))
        .limit(2)
        .all(db)
        .await?;
    if visible.len() != 1 {
        return Err(OperationError::NotFound.into());
    }
    let initial = visible.first().ok_or(OperationError::NotFound)?;
    let actor = AdoptionActor::capture(db, principal, initial.person_id).await?;
    let frozen = legacy_prescription::Entity::find_by_id(initial.schedule_id)
        .lock_exclusive()
        .one(db)
        .await?;
    actor.current(db, principal, settings).await?;
    let frozen = frozen.ok_or(OperationError::Forbidden)?;
    let visible_ids = legacy_prescription::Entity::find()
        .select_only()
        .column(legacy_prescription::Column::ScheduleId)
        .filter(legacy_prescription::Column::SourcePortableId.eq(portable_id))
        .limit(2)
        .into_tuple::<i64>()
        .all(db)
        .await?;
    if visible_ids.as_slice() != [frozen.schedule_id] {
        return Err(OperationError::NotFound.into());
    }
    if frozen.person_id != actor.subject.get() || frozen.source_portable_id != portable_id {
        return Err(OperationError::Forbidden.into());
    }
    if let Some(id) = frozen.adopted_person_schedule_id.as_ref() {
        let state = shared_schedule::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or(OperationError::NotFound)?;
        actor.current(db, principal, settings).await?;
        return SharedPlan::try_from(&state).map_err(Into::into);
    }
    validate(&frozen)?;
    let configured_zone = super::super::doses::app_zone(settings);
    let zone = match frozen.origin_time_zone.as_deref() {
        Some(zone) => zone
            .parse::<Tz>()
            .map_err(|_| OperationError::Unavailable)?,
        None => configured_zone,
    };
    let now = Utc::now().naive_utc();
    let mut canonical = canonical(&frozen, zone, now);
    let input = super::super::treatments::ScheduleInput {
        dose_amount: frozen.dose_amount,
        dose_unit: frozen.dose_unit.clone(),
        frequency: frozen.frequency.clone(),
        start_date: frozen.start_date,
        end_date: frozen.end_date,
        max_daily_doses: frozen.max_daily_doses,
        min_hours_between_doses: Some(frozen.min_hours_between_doses),
        dose_cycle: frozen
            .dose_cycle
            .and_then(crate::models::care::treatment_command::DoseCycle::from_database),
        schedule_type: crate::models::care::treatment_command::ScheduleType::from_database(
            frozen.schedule_type,
        ),
        schedule_config: Some(frozen.schedule_config.clone()),
        ..Default::default()
    }
    .validated()?;
    super::super::treatments::amend_personal(&mut canonical, &input)?;
    purpose(db, &frozen, &actor, zone).await?;
    actor.current(db, principal, settings).await?;
    person_schedule::Entity::insert(canonical.clone().into_active_model().reset_all())
        .exec_without_returning(db)
        .await?;
    actor.current(db, principal, settings).await?;
    super::projection::publish(db, &canonical).await?;
    actor.current(db, principal, settings).await?;
    let changed = legacy_prescription::Entity::update_many()
        .col_expr(
            legacy_prescription::Column::AdoptedPersonScheduleId,
            Expr::val(canonical.id.clone()),
        )
        .filter(legacy_prescription::Column::ScheduleId.eq(frozen.schedule_id))
        .filter(legacy_prescription::Column::AdoptedPersonScheduleId.is_null())
        .exec(db)
        .await?;
    if changed.rows_affected != 1 {
        return Err(OperationError::Forbidden.into());
    }
    actor.current(db, principal, settings).await?;
    let state = shared_schedule::Entity::find_by_id(&canonical.id)
        .one(db)
        .await?
        .ok_or(OperationError::Unavailable)?;
    let safe = SharedPlan::try_from(&state)?;
    adoption_audit(db, &actor, &safe, request_id, principal.provenance(), now).await?;
    Ok(safe)
}

fn validate(frozen: &legacy_prescription::Model) -> Result<(), OperationError> {
    use crate::models::care::{
        stock_identity::ExactProductIdentity,
        treatment_command::{DoseCycle, ScheduleType},
    };
    if ExactProductIdentity::from_parts(
        frozen.dmd_system.as_deref(),
        frozen.dmd_concept_class.as_deref(),
        frozen.dmd_code.as_deref(),
        frozen.stock_unit.as_deref(),
    )
    .is_none()
        || frozen
            .dose_amount
            .is_none_or(|amount| amount <= sea_orm::prelude::Decimal::ZERO)
        || frozen
            .dose_unit
            .as_deref()
            .is_none_or(|unit| unit.trim().is_empty())
        || ScheduleType::from_database(frozen.schedule_type).is_none()
        || frozen
            .dose_cycle
            .is_some_and(|cycle| DoseCycle::from_database(cycle).is_none())
        || (frozen.pause_instance_id.is_some() && frozen.pause_started_at.is_none())
    {
        return Err(crate::models::profile::validation(
            "source_portable_id",
            "The frozen prescription has unknown required facts and cannot be adopted",
        ));
    }
    Ok(())
}

fn canonical(
    frozen: &legacy_prescription::Model,
    zone: Tz,
    now: chrono::NaiveDateTime,
) -> person_schedule::Model {
    person_schedule::Model {
        id: Uuid::new_v4().to_string(),
        person_id: frozen.person_id,
        origin_household_id: frozen.household_id,
        origin_schedule_id: frozen.schedule_id,
        origin_stock_id: frozen.origin_stock_id,
        origin_stock_portable_id: frozen.origin_stock_portable_id.clone(),
        medicine_name: frozen.medicine_name.clone(),
        dmd_system: frozen.dmd_system.clone(),
        dmd_code: frozen.dmd_code.clone(),
        dmd_concept_class: frozen.dmd_concept_class.clone(),
        stock_unit: frozen.stock_unit.clone(),
        stock_dose_default: frozen.stock_dose_default,
        stock_default_known: frozen.stock_default_known,
        pause_started_at: frozen.pause_started_at,
        pause_instance_id: frozen.pause_instance_id.clone(),
        pause_reason: None,
        pause_note: None,
        origin_time_zone: zone.name().into(),
        timezone_provenance: if frozen.origin_time_zone.is_some() {
            "preserved_origin_zone"
        } else {
            "adopted_application_zone"
        }
        .into(),
        clinical_revision: 1,
        active: frozen.active,
        retired_at: frozen.retired_at,
        dose_amount: frozen.dose_amount,
        dose_unit: frozen.dose_unit.clone(),
        dose_cycle: frozen.dose_cycle,
        frequency: frozen.frequency.clone(),
        start_date: frozen.start_date,
        end_date: frozen.end_date,
        notes: None,
        max_daily_doses: frozen.max_daily_doses,
        min_hours_between_doses: frozen.min_hours_between_doses,
        schedule_type: frozen.schedule_type,
        schedule_config: frozen.schedule_config.clone(),
        created_at: frozen.source_created_at,
        updated_at: now,
    }
}

async fn purpose(
    db: &DatabaseTransaction,
    frozen: &legacy_prescription::Model,
    actor: &AdoptionActor,
    zone: Tz,
) -> Result<(), OperationError> {
    let declaration = actor
        .declaration()
        .map_or_else(|| "self".into(), |id| id.to_string());
    let query = Query::select()
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.legacy_adoption_source_id"),
            Expr::val(frozen.schedule_id.to_string()),
            Expr::val(true),
        ]))
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.legacy_adoption_declaration_id"),
            Expr::val(declaration),
            Expr::val(true),
        ]))
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.legacy_adoption_zone"),
            Expr::val(zone.name()),
            Expr::val(true),
        ]))
        .to_owned();
    db.execute_raw(db.get_database_backend().build(&query))
        .await?;
    Ok(())
}

async fn adoption_audit(
    db: &DatabaseTransaction,
    actor: &AdoptionActor,
    safe: &SharedPlan,
    request_id: &str,
    provenance: &CredentialProvenance,
    now: chrono::NaiveDateTime,
) -> Result<(), OperationError> {
    version::Entity::insert(version::ActiveModel {
        item_type: Set("LegacyPrescriptionAdoption".into()), item_id: Set(actor.subject.get()), event: Set("adopt".into()),
        object_changes: Set(Some(serde_json::to_string(safe).map_err(|_| OperationError::Unavailable)?)),
        whodunnit: Set(Some(actor.account.get().to_string())), request_id: Set(Some(request_id.into())), created_at: Set(Some(now)),
        audit_context: Set(serde_json::json!({"actor_account_id":actor.account.get(),"actor_person_id":actor.person.get(),"subject_person_id":actor.subject.get(),
            "guardian_declaration_id":actor.declaration(),"person_schedule_id":safe.id,"household_id":null,"request_id":request_id,
            "authentication_method":provenance.method.label(),"session_reference":format!("{}:{}",provenance.method.reference_prefix(),provenance.reference),"policy_class":"LegacyPrescriptionAdoption"})),
        ..Default::default()
    }).exec_without_returning(db).await?;
    Ok(())
}
