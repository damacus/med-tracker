use super::{ClinicalRevision, PersonScheduleId, PersonalAction, SharedPlan, sharing, view};
use crate::models::{
    access::{self, TenantTransaction},
    care::{medications, treatments},
    entities::{account, person_schedule, shared_schedule, version},
    errors::OperationError,
    identity::resource::{AuthenticationError, ValidatedPrincipal},
    ids::{AccountId, MedicationId, PersonId},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, EntityTrait,
    IntoActiveModel, QueryFilter, QuerySelect, Set, TransactionTrait,
    sea_query::{Expr, ExprTrait, Func, Query},
};
use serde_json::{Value, json};

pub(crate) enum Failure {
    Operation(OperationError),
    Authentication(AuthenticationError),
}
impl From<OperationError> for Failure {
    fn from(error: OperationError) -> Self {
        Self::Operation(error)
    }
}
impl From<AuthenticationError> for Failure {
    fn from(error: AuthenticationError) -> Self {
        Self::Authentication(error)
    }
}
impl From<sea_orm::DbErr> for Failure {
    fn from(error: sea_orm::DbErr) -> Self {
        Self::Operation(error.into())
    }
}

struct ClinicalActor {
    account: AccountId,
    person: PersonId,
    subject: PersonId,
    authority: sharing::ConsentAuthority,
}

impl ClinicalActor {
    async fn capture(
        db: &DatabaseTransaction,
        principal: &ValidatedPrincipal,
        id: &PersonScheduleId,
    ) -> Result<(Self, shared_schedule::Model), Failure> {
        let actual = account::Entity::find_by_id(principal.account_id())
            .one(db)
            .await?
            .ok_or(OperationError::Unauthenticated)?;
        let initial = shared_schedule::Entity::find_by_id(id.as_str())
            .one(db)
            .await?
            .ok_or(OperationError::NotFound)?;
        let subject = PersonId::from_database(initial.person_id);
        let authority = if actual.person_id == initial.person_id {
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
        Ok((
            Self {
                account: AccountId::from_database(actual.id),
                person: PersonId::from_database(actual.person_id),
                subject,
                authority,
            },
            initial,
        ))
    }

    async fn current(&self, db: &DatabaseTransaction) -> Result<(), OperationError> {
        access::verify_account(db, self.account.get()).await?;
        let actual = account::Entity::find_by_id(self.account.get())
            .one(db)
            .await?
            .ok_or(OperationError::Unauthenticated)?;
        if actual.person_id != self.person.get()
            || !self.authority.current(db, self.subject).await?
        {
            return Err(OperationError::Forbidden);
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

enum Context<'a> {
    Account(Option<&'a Value>),
    Household(&'a TenantTransaction),
}
impl Context<'_> {
    async fn revalidate(
        &self,
        db: &DatabaseTransaction,
        principal: &ValidatedPrincipal,
        actor: &ClinicalActor,
    ) -> Result<(), Failure> {
        match self {
            Self::Account(settings) => principal.revalidate_account(db, *settings).await?,
            Self::Household(tenant) => principal.revalidate_household(tenant).await?,
        }
        actor.current(db).await?;
        Ok(())
    }
    fn household(&self) -> Option<i64> {
        match self {
            Self::Account(_) => None,
            Self::Household(tenant) => Some(tenant.scope().household_id),
        }
    }
    fn membership(&self) -> Option<i64> {
        match self {
            Self::Account(_) => None,
            Self::Household(tenant) => Some(tenant.membership().id),
        }
    }
}

#[derive(Clone, Copy)]
enum Change {
    Edit,
    Pause,
    Resume,
    Retire,
    Medicine,
}
impl Change {
    const fn label(self) -> &'static str {
        match self {
            Self::Edit => "update",
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::Retire => "retire",
            Self::Medicine => "change_medicine",
        }
    }
}

async fn lock(
    db: &DatabaseTransaction,
    principal: &ValidatedPrincipal,
    id: &PersonScheduleId,
    expected: ClinicalRevision,
    context: &Context<'_>,
) -> Result<(ClinicalActor, shared_schedule::Model), Failure> {
    let (actor, initial) = ClinicalActor::capture(db, principal, id).await?;
    let row = shared_schedule::Entity::find_by_id(id.as_str())
        .lock_exclusive()
        .one(db)
        .await?;
    context.revalidate(db, principal, &actor).await?;
    let row = row.ok_or(OperationError::NotFound)?;
    if row.person_id != initial.person_id {
        return Err(OperationError::Forbidden.into());
    }
    if row.clinical_revision != expected.get() {
        return Err(stale().into());
    }
    if row.retired_at.is_some() {
        return Err(OperationError::NotFound.into());
    }
    Ok((actor, row))
}

fn stale() -> OperationError {
    OperationError::Conflict {
        code: "conflict".into(),
        details: json!({"error":"Prescription changed. Review the current prescription."}),
    }
}

pub(crate) async fn update(
    db: &DatabaseTransaction,
    principal: &ValidatedPrincipal,
    id: PersonScheduleId,
    expected: ClinicalRevision,
    action: PersonalAction,
    request_id: &str,
    settings: Option<&Value>,
) -> Result<SharedPlan, Failure> {
    let context = Context::Account(settings);
    let (actor, before) = lock(db, principal, &id, expected, &context).await?;
    let mut after = before.clone();
    let now = Utc::now().naive_utc();
    let change = match action {
        PersonalAction::Edit(input) => {
            treatments::amend_shared(&mut after, &input)?;
            Change::Edit
        }
        PersonalAction::Pause => {
            if after.active {
                after.pause_started_at = Some(now);
                after.pause_instance_id = Some(uuid::Uuid::new_v4().to_string());
            }
            after.active = false;
            Change::Pause
        }
        PersonalAction::Resume => {
            after.active = true;
            after.pause_started_at = None;
            after.pause_instance_id = None;
            after.last_closed_pause_instance_id = before
                .pause_instance_id
                .clone()
                .or(before.last_closed_pause_instance_id.clone());
            Change::Resume
        }
        PersonalAction::Retire => {
            after.active = false;
            after.retired_at = Some(now);
            Change::Retire
        }
    };
    write(
        db,
        principal,
        &actor,
        ClinicalChange {
            before,
            after,
            kind: change,
            at: now,
            stock: None,
        },
        &context,
        request_id,
    )
    .await
}

pub(crate) struct MedicineChange {
    medication: MedicationId,
    etag: String,
    amount: sea_orm::prelude::Decimal,
    unit: String,
}
impl MedicineChange {
    pub(crate) fn parse(
        medication: i64,
        etag: String,
        amount: &str,
        unit: String,
    ) -> Result<Self, OperationError> {
        if medication <= 0 || etag.is_empty() {
            return Err(crate::models::profile::validation(
                "medication",
                "is required",
            ));
        }
        if !treatments::supported_unit(&unit) {
            return Err(crate::models::profile::validation(
                "dose_unit",
                "is not supported",
            ));
        }
        Ok(Self {
            medication: MedicationId::from_database(medication),
            etag,
            amount: treatments::prescribed_amount(amount)?,
            unit,
        })
    }
}

pub(crate) async fn medicine(
    tenant: &TenantTransaction,
    principal: &ValidatedPrincipal,
    id: PersonScheduleId,
    expected: ClinicalRevision,
    choice: MedicineChange,
) -> Result<SharedPlan, Failure> {
    let db = tenant.transaction();
    if principal.account_id() != tenant.scope().actor.account_id {
        return Err(OperationError::Forbidden.into());
    }
    super::super::sync::lock(tenant).await?;
    let context = Context::Household(tenant);
    let (actor, before) = lock(db, principal, &id, expected, &context).await?;
    access::require_person_access(tenant, before.person_id, access::PersonAccess::Manage).await?;
    let stock = access::medication_scope(tenant)
        .filter(crate::models::entities::medication::Column::Id.eq(choice.medication.get()))
        .lock_shared()
        .one(db)
        .await?
        .ok_or(OperationError::NotFound)?;
    context.revalidate(db, principal, &actor).await?;
    let snapshot = medications::read_stock_snapshot(tenant, &stock.id.to_string()).await?;
    if snapshot.etag != choice.etag {
        return Err(stale().into());
    }
    if crate::models::care::stock_identity::ExactProductIdentity::from_stock(&stock).is_none()
        || stock.dose_unit.as_deref() != Some(choice.unit.as_str())
    {
        return Err(crate::models::profile::validation(
            "medication",
            "Choose the exact prescribed product and confirm its current unit",
        )
        .into());
    }
    let mut after = before.clone();
    after.medicine_name = stock.name.clone();
    after.dmd_system = stock.dmd_system.clone();
    after.dmd_code = stock.dmd_code.clone();
    after.dmd_concept_class = stock.dmd_concept_class.clone();
    after.stock_unit = stock.dose_unit.clone();
    after.stock_dose_default = stock
        .dose_amount
        .and_then(|amount| amount.to_string().parse().ok());
    after.stock_default_known = stock.dose_amount.is_none() || after.stock_dose_default.is_some();
    after.dose_amount = Some(choice.amount);
    after.dose_unit = Some(choice.unit);
    write(
        db,
        principal,
        &actor,
        ClinicalChange {
            before,
            after,
            kind: Change::Medicine,
            at: Utc::now().naive_utc(),
            stock: Some(choice.medication),
        },
        &context,
        &tenant.scope().request_id,
    )
    .await
}

struct ClinicalChange {
    before: shared_schedule::Model,
    after: shared_schedule::Model,
    kind: Change,
    at: chrono::NaiveDateTime,
    stock: Option<MedicationId>,
}

async fn write(
    db: &DatabaseTransaction,
    principal: &ValidatedPrincipal,
    actor: &ClinicalActor,
    change: ClinicalChange,
    context: &Context<'_>,
    request_id: &str,
) -> Result<SharedPlan, Failure> {
    let ClinicalChange {
        before,
        mut after,
        kind: change,
        at: now,
        stock,
    } = change;
    if before == after {
        return SharedPlan::try_from(&before).map_err(Into::into);
    }
    after.clinical_revision = before
        .clinical_revision
        .checked_add(1)
        .ok_or(OperationError::Unavailable)?;
    after.updated_at = now;
    let previous = scope_values(db).await?;
    let declaration = actor
        .declaration()
        .map_or_else(|| "self".into(), |id| id.to_string());
    clinical_scope(
        db,
        &before.id,
        &declaration,
        &stock.map(|id| id.get().to_string()).unwrap_or_default(),
    )
    .await?;
    let savepoint = db.begin().await?;
    let update = canonical_patch(&before, &after, change)
        .exec(&savepoint)
        .await;
    match update {
        Ok(result) => {
            savepoint.commit().await?;
            context.revalidate(db, principal, actor).await?;
            if result.rows_affected != 1 {
                return Err(OperationError::NotFound.into());
            }
        }
        Err(error) => {
            savepoint.rollback().await?;
            context.revalidate(db, principal, actor).await?;
            return Err(error.into());
        }
    }
    clinical_scope(db, &previous.0, &previous.1, &previous.2).await?;
    after
        .clone()
        .into_active_model()
        .reset_all()
        .update(db)
        .await?;
    super::pauses::record_resume(
        db,
        before.pause_instance_id.as_deref(),
        &after.id,
        super::pauses::ResumeActor {
            account: actor.account,
            membership: context.membership(),
            request_id,
            provenance: Some(principal.provenance()),
        },
    )
    .await?;
    let provenance = principal.provenance();
    version::Entity::insert(version::ActiveModel{
        item_type:Set("SharedPersonSchedule".into()),item_id:Set(actor.subject.get()),event:Set(change.label().into()),
        object_changes:Set(Some(view::clinical_changes(&before,&after)?.to_string())),whodunnit:Set(Some(actor.account.get().to_string())),
        household_id:Set(context.household()),actor_membership_id:Set(context.membership()),created_at:Set(Some(now)),request_id:Set(Some(request_id.into())),
        audit_context:Set(json!({"actor_account_id":actor.account.get(),"actor_person_id":actor.person.get(),"actor_membership_id":context.membership(),
            "person_id":actor.subject.get(),"person_schedule_id":after.id,"guardian_declaration_id":actor.declaration(),"previous_clinical_revision":before.clinical_revision,
            "clinical_revision":after.clinical_revision,"request_id":request_id,"authentication_method":provenance.method.label(),
            "session_reference":format!("{}:{}",provenance.method.reference_prefix(),provenance.reference)})),..Default::default()
    }).exec_without_returning(db).await?;
    context.revalidate(db, principal, actor).await?;
    SharedPlan::try_from(&after).map_err(Into::into)
}

fn canonical_patch(
    before: &shared_schedule::Model,
    after: &shared_schedule::Model,
    change: Change,
) -> sea_orm::UpdateMany<person_schedule::Entity> {
    let mut query = person_schedule::Entity::update_many()
        .col_expr(
            person_schedule::Column::ClinicalRevision,
            Expr::val(after.clinical_revision),
        )
        .col_expr(
            person_schedule::Column::UpdatedAt,
            Expr::val(after.updated_at),
        );
    macro_rules! field {
        ($name:ident,$column:ident) => {
            if before.$name != after.$name {
                query = query.col_expr(person_schedule::Column::$column, Expr::val(after.$name));
            }
        };
    }
    macro_rules! owned {
        ($name:ident,$column:ident) => {
            if before.$name != after.$name {
                query = query.col_expr(
                    person_schedule::Column::$column,
                    Expr::val(after.$name.clone()),
                );
            }
        };
    }
    field!(active, Active);
    field!(retired_at, RetiredAt);
    field!(dose_amount, DoseAmount);
    owned!(dose_unit, DoseUnit);
    field!(dose_cycle, DoseCycle);
    owned!(frequency, Frequency);
    field!(start_date, StartDate);
    field!(end_date, EndDate);
    field!(max_daily_doses, MaxDailyDoses);
    field!(min_hours_between_doses, MinHoursBetweenDoses);
    field!(schedule_type, ScheduleType);
    owned!(schedule_config, ScheduleConfig);
    owned!(origin_time_zone, OriginTimeZone);
    field!(pause_started_at, PauseStartedAt);
    owned!(pause_instance_id, PauseInstanceId);
    owned!(medicine_name, MedicineName);
    owned!(dmd_system, DmdSystem);
    owned!(dmd_code, DmdCode);
    owned!(dmd_concept_class, DmdConceptClass);
    owned!(stock_unit, StockUnit);
    field!(stock_dose_default, StockDoseDefault);
    field!(stock_default_known, StockDefaultKnown);
    if before.origin_time_zone != after.origin_time_zone {
        query = query.col_expr(
            person_schedule::Column::TimezoneProvenance,
            Expr::val("authorised_correction"),
        );
    }
    if matches!(change, Change::Pause | Change::Resume) && before.active != after.active {
        query = query
            .col_expr(
                person_schedule::Column::PauseReason,
                Expr::val(Option::<String>::None),
            )
            .col_expr(
                person_schedule::Column::PauseNote,
                Expr::val(Option::<String>::None),
            );
    }
    query.filter(
        Expr::expr(Func::cust("current_setting").args([
            Expr::val("med_tracker.sharing_clinical_plan_id"),
            Expr::val(true),
        ]))
        .eq(&before.id),
    )
}

async fn scope_values(
    db: &DatabaseTransaction,
) -> Result<(String, String, String), OperationError> {
    let mut query = Query::select();
    for (key, name) in [
        ("plan", "med_tracker.sharing_clinical_plan_id"),
        ("declaration", "med_tracker.sharing_clinical_declaration_id"),
        ("stock", "med_tracker.sharing_clinical_stock_id"),
    ] {
        query.expr_as(
            Func::cust("current_setting").args([Expr::val(name), Expr::val(true)]),
            key,
        );
    }
    let row = db
        .query_one_raw(db.get_database_backend().build(&query.to_owned()))
        .await?
        .ok_or(OperationError::Unavailable)?;
    Ok((
        row.try_get::<Option<String>>("", "plan")?
            .unwrap_or_default(),
        row.try_get::<Option<String>>("", "declaration")?
            .unwrap_or_default(),
        row.try_get::<Option<String>>("", "stock")?
            .unwrap_or_default(),
    ))
}

async fn clinical_scope(
    db: &DatabaseTransaction,
    plan: &str,
    declaration: &str,
    stock: &str,
) -> Result<(), OperationError> {
    let query = Query::select()
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.sharing_clinical_plan_id"),
            Expr::val(plan),
            Expr::val(true),
        ]))
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.sharing_clinical_declaration_id"),
            Expr::val(declaration),
            Expr::val(true),
        ]))
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.sharing_clinical_stock_id"),
            Expr::val(stock),
            Expr::val(true),
        ]))
        .to_owned();
    db.execute_raw(db.get_database_backend().build(&query))
        .await?;
    Ok(())
}
