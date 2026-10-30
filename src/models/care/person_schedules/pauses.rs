use super::*;
use crate::models::{
    entities::{account, shared_pause_closure, shared_schedule},
    ids::AccountId,
};

pub(super) struct ResumeActor<'a> {
    pub(super) account: AccountId,
    pub(super) membership: Option<i64>,
    pub(super) request_id: &'a str,
    pub(super) provenance: Option<&'a super::super::doses::CredentialProvenance>,
}

pub(super) async fn record_resume(
    db: &DatabaseTransaction,
    previous_instance: Option<&str>,
    plan: &str,
    actor: ResumeActor<'_>,
) -> Result<(), OperationError> {
    let Some(instance) = previous_instance else {
        return Ok(());
    };
    let state = shared_schedule::Entity::find_by_id(plan)
        .one(db)
        .await?
        .ok_or(OperationError::NotFound)?;
    if !state.active
        || state.pause_instance_id.is_some()
        || state.last_closed_pause_instance_id.as_deref() != Some(instance)
    {
        return Ok(());
    }
    let account = account::Entity::find_by_id(actor.account.get())
        .one(db)
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    let declaration = if account.person_id == state.person_id {
        None
    } else {
        Some(
            sharing::guardian(
                db,
                crate::models::ids::PersonId::from_database(state.person_id),
            )
            .await?
            .ok_or(OperationError::Forbidden)?,
        )
    };
    let method = actor
        .provenance
        .map(|value| value.method.label().to_owned());
    let reference = actor
        .provenance
        .map(|value| format!("{}:{}", value.method.reference_prefix(), value.reference));
    shared_pause_closure::Entity::insert(shared_pause_closure::ActiveModel {
        pause_instance_id: Set(instance.into()),
        person_schedule_id: Set(plan.into()),
        clinical_revision: Set(state.clinical_revision),
        ended_at: Set(state.updated_at),
        actor_account_id: Set(account.id),
        actor_person_id: Set(account.person_id),
        actor_membership_id: Set(actor.membership),
        guardian_declaration_id: Set(declaration),
        request_id: Set(actor.request_id.into()),
        authentication_method: Set(method),
        session_reference: Set(reference),
        created_at: Set(Utc::now().naive_utc()),
    })
    .exec_without_returning(db)
    .await?;
    Ok(())
}

pub(super) async fn associate_start(
    db: &DatabaseTransaction,
    source: &schedule::Model,
) -> Result<(), OperationError> {
    if source.execution_ended_at.is_some() {
        return Ok(());
    }
    let Some(plan) = source.person_schedule_id.as_ref() else {
        return Ok(());
    };
    let state = shared_schedule::Entity::find_by_id(plan)
        .one(db)
        .await?
        .ok_or(OperationError::NotFound)?;
    let Some(instance) = state.pause_instance_id.as_ref() else {
        return Ok(());
    };
    let Some(period) = crate::models::entities::pause_period::Entity::find()
        .filter(crate::models::entities::pause_period::Column::ScheduleId.eq(source.id))
        .filter(crate::models::entities::pause_period::Column::PortableId.eq(instance))
        .filter(crate::models::entities::pause_period::Column::EndedAt.is_null())
        .one(db)
        .await?
    else {
        return Ok(());
    };
    if let Some(existing) = period.canonical_person_schedule_id.as_ref() {
        if existing != plan
            || period.started_at != state.pause_started_at
            || period
                .canonical_pause_revision
                .is_none_or(|revision| revision > state.clinical_revision)
        {
            return Err(OperationError::Unavailable);
        }
        return Ok(());
    }
    let changed = crate::models::entities::pause_period::Entity::update_many()
        .col_expr(
            crate::models::entities::pause_period::Column::CanonicalPersonScheduleId,
            sea_orm::sea_query::Expr::val(plan.clone()),
        )
        .col_expr(
            crate::models::entities::pause_period::Column::CanonicalPauseRevision,
            sea_orm::sea_query::Expr::val(state.clinical_revision),
        )
        .filter(crate::models::entities::pause_period::Column::Id.eq(period.id))
        .filter(crate::models::entities::pause_period::Column::CanonicalPersonScheduleId.is_null())
        .exec(db)
        .await?;
    if changed.rows_affected != 1 {
        return Err(OperationError::Unavailable);
    }
    Ok(())
}

pub(super) async fn project_owned(
    db: &DatabaseTransaction,
    source: &schedule::Model,
) -> Result<(), OperationError> {
    if source.execution_ended_at.is_some() {
        return Ok(());
    }
    let Some(plan) = source.person_schedule_id.as_ref() else {
        return Ok(());
    };
    let Some(period) = crate::models::entities::pause_period::Entity::find()
        .filter(crate::models::entities::pause_period::Column::ScheduleId.eq(source.id))
        .filter(crate::models::entities::pause_period::Column::CanonicalPersonScheduleId.eq(plan))
        .filter(crate::models::entities::pause_period::Column::EndedAt.is_null())
        .lock_exclusive()
        .one(db)
        .await?
    else {
        return Ok(());
    };
    let Some(closure) = shared_pause_closure::Entity::find_by_id(&period.portable_id)
        .filter(shared_pause_closure::Column::PersonScheduleId.eq(plan))
        .one(db)
        .await?
    else {
        return Ok(());
    };
    let mut closed = period.into_active_model();
    closed.ended_at = Set(Some(closure.ended_at));
    closed.resumed_by_membership_id = Set(None);
    closed.canonical_resume_id = Set(Some(closure.pause_instance_id));
    closed.updated_at = Set(closure.ended_at);
    closed.update(db).await?;
    Ok(())
}
