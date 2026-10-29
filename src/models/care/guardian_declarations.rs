use crate::models::{
    access::{self, TenantTransaction},
    care::doses::CredentialProvenance,
    entities::{account, person, security_audit_event, version},
    errors::OperationError,
};
use chrono::{NaiveDate, Utc};
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, DbBackend, EntityTrait, QuerySelect, Set, Statement,
};
use serde_json::json;

#[derive(serde::Serialize)]
pub(crate) struct OwnDeclaration {
    pub id: String,
    pub subject_person_id: String,
    pub subject_name_at_declaration: String,
    pub statement_version: i16,
    pub source: String,
    pub declared_at: chrono::DateTime<Utc>,
    pub withdrawn_at: Option<chrono::DateTime<Utc>>,
    pub superseded_at: Option<chrono::DateTime<Utc>>,
    pub revision: String,
}

impl OwnDeclaration {
    fn decode(row: &sea_orm::QueryResult) -> Result<Self, OperationError> {
        Ok(Self {
            id: row.try_get::<i64>("", "id")?.to_string(),
            subject_person_id: row.try_get::<i64>("", "subject_person_id")?.to_string(),
            subject_name_at_declaration: row.try_get("", "subject_name_at_declaration")?,
            statement_version: row.try_get("", "statement_version")?,
            source: row.try_get("", "source")?,
            declared_at: row.try_get("", "declared_at")?,
            withdrawn_at: row.try_get("", "withdrawn_at")?,
            superseded_at: row.try_get("", "superseded_at")?,
            revision: row.try_get::<i64>("", "revision")?.to_string(),
        })
    }
}

pub(crate) struct OwnPage {
    pub records: Vec<OwnDeclaration>,
    pub next_before_id: Option<String>,
}

pub(crate) async fn own_page(
    transaction: &sea_orm::DatabaseTransaction,
    account_id: i64,
    limit: u16,
    before_id: Option<i64>,
) -> Result<OwnPage, OperationError> {
    if !(1..=100).contains(&limit) || before_id.is_some_and(|id| id <= 0) {
        return Err(validation("pagination", "is invalid"));
    }
    let rows = transaction.query_all_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT d.* FROM public.guardian_declarations d JOIN public.accounts a ON a.person_id=d.guardian_person_id WHERE a.id=$1 AND ($2::bigint IS NULL OR d.id<$2) ORDER BY d.id DESC LIMIT $3",
        [account_id.into(), before_id.into(), (i32::from(limit)+1).into()],
    )).await?;
    let mut records = rows
        .iter()
        .map(OwnDeclaration::decode)
        .collect::<Result<Vec<_>, _>>()?;
    let more = records.len() > usize::from(limit);
    records.truncate(usize::from(limit));
    let next_before_id = if more {
        records.last().map(|row| row.id.clone())
    } else {
        None
    };
    Ok(OwnPage {
        records,
        next_before_id,
    })
}

pub(crate) async fn withdraw_own(
    transaction: &sea_orm::DatabaseTransaction,
    account_id: i64,
    declaration_id: i64,
    expected_revision: i64,
    request_id: &str,
    provenance: &CredentialProvenance,
) -> Result<OwnDeclaration, OperationError> {
    let row = transaction.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT d.* FROM public.guardian_declarations d JOIN public.accounts a ON a.person_id=d.guardian_person_id WHERE d.id=$1 AND a.id=$2 FOR UPDATE OF d",
        [declaration_id.into(),account_id.into()],
    )).await?.ok_or(OperationError::NotFound)?;
    let current = OwnDeclaration::decode(&row)?;
    if expected_revision != 1 {
        return Err(OperationError::Conflict {
            code: "stale_guardian_declaration".into(),
            details: json!({}),
        });
    }
    if current.superseded_at.is_some() {
        return Err(OperationError::Conflict {
            code: "guardian_declaration_superseded".into(),
            details: json!({}),
        });
    }
    if current.withdrawn_at.is_some() {
        return Ok(current);
    }
    super::person_schedules::sharing::withdraw_declaration(
        transaction,
        crate::models::ids::AccountId::from_database(account_id),
        declaration_id,
        request_id,
        provenance,
    )
    .await?;
    let updated = transaction.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "UPDATE public.guardian_declarations SET withdrawn_at=pg_catalog.clock_timestamp(),revision=revision+1 WHERE id=$1 RETURNING *",
        [declaration_id.into()],
    )).await?.ok_or(OperationError::NotFound)?;
    let result = OwnDeclaration::decode(&updated)?;
    let actor = crate::models::entities::account::Entity::find_by_id(account_id)
        .one(transaction)
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    let person_id = actor.person_id;
    version::ActiveModel {
        item_type: Set("GuardianDeclaration".into()),
        item_id: Set(declaration_id),
        event: Set("withdraw".into()),
        object: Set(Some(serde_json::to_string(&current).map_err(|_| OperationError::Unavailable)?)),
        object_changes: Set(Some(serde_json::to_string(&result).map_err(|_| OperationError::Unavailable)?)),
        whodunnit: Set(Some(account_id.to_string())),
        request_id: Set(Some(request_id.to_owned())),
        household_id: Set(None),
        actor_membership_id: Set(None),
        audit_context: Set(json!({"actor_account_id":account_id,"actor_person_id":person_id,"household_id":null,"request_id":request_id,"authentication_method":provenance.method.label(),"session_reference":format!("{}:{}",provenance.method.reference_prefix(),provenance.reference),"policy_class":"GuardianDeclaration","policy_query":"withdraw_own"})),
        created_at: Set(Some(Utc::now().naive_utc())),
        ..Default::default()
    }.insert(transaction).await?;
    Ok(result)
}

pub(crate) struct InitialGuardianAttestation(());

impl InitialGuardianAttestation {
    pub(crate) fn parse(value: Option<bool>) -> Result<Self, OperationError> {
        if value == Some(true) {
            Ok(Self(()))
        } else {
            Err(validation(
                "initial_guardian_declaration",
                "must be explicitly accepted",
            ))
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct DeclarationId(i64);

impl DeclarationId {
    pub(crate) const fn from_database(value: i64) -> Self {
        Self(value)
    }
    pub(crate) const fn get(self) -> i64 {
        self.0
    }
}

pub(crate) fn validation(field: &str, message: &str) -> OperationError {
    OperationError::Validation {
        details: json!({"errors":{field:[message]}}),
    }
}

pub(crate) async fn lock_creator_account(tenant: &TenantTransaction) -> Result<(), OperationError> {
    account::Entity::find_by_id(tenant.scope().actor.account_id)
        .lock_shared()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    access::verify_account(tenant.transaction(), tenant.scope().actor.account_id).await
}

pub(crate) async fn eligible_actor(
    tenant: &TenantTransaction,
    today: NaiveDate,
) -> Result<person::Model, OperationError> {
    access::recheck(tenant).await?;
    let actor = access::person_queries::account_person(tenant.scope().actor)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Forbidden)?;
    if actor.id != tenant.membership().person_id
        || actor.person_type != 0
        || !actor.has_capacity
        || actor
            .date_of_birth
            .is_some_and(|date| super::people::age(date, today) < 18)
    {
        return Err(OperationError::Forbidden);
    }
    Ok(actor)
}

pub(crate) async fn import_manageable_subjects(
    tenant: &TenantTransaction,
    today: NaiveDate,
) -> Result<Vec<i64>, OperationError> {
    let guardian = eligible_actor(tenant, today).await?;
    let rows = tenant.transaction().query_all_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT d.subject_person_id,p.person_type,p.date_of_birth,p.has_capacity FROM public.guardian_declarations d JOIN public.carer_relationships c ON c.guardian_declaration_id=d.id AND c.carer_id=d.guardian_person_id AND c.patient_id=d.subject_person_id JOIN public.people p ON p.id=d.subject_person_id JOIN public.household_memberships m ON m.person_id=p.id AND m.household_id=c.household_id WHERE d.guardian_person_id=$1 AND d.withdrawn_at IS NULL AND d.superseded_at IS NULL AND c.household_id=$2 AND c.active AND m.status='active' AND m.revoked_at IS NULL ORDER BY d.id FOR SHARE OF d",
        [guardian.id.into(),tenant.scope().household_id.into()],
    )).await?;
    let mut ids = Vec::new();
    for row in rows {
        let kind: i32 = row.try_get("", "person_type")?;
        let birth: Option<NaiveDate> = row.try_get("", "date_of_birth")?;
        let capacity: bool = row.try_get("", "has_capacity")?;
        if !capacity
            && birth.is_some_and(|birth| {
                let years = super::people::age(birth, today);
                years >= 0 && matches!((kind, years < 18), (1, true) | (2, false))
            })
        {
            ids.push(row.try_get("", "subject_person_id")?);
        }
    }
    Ok(ids)
}

pub(crate) async fn manageable_subjects(
    tenant: &TenantTransaction,
) -> Result<Vec<i64>, OperationError> {
    access::recheck(tenant).await?;
    let owner = account::Entity::find_by_id(tenant.scope().actor.account_id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    let locked = tenant.transaction().query_all_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT d.id FROM public.guardian_declarations d JOIN public.carer_relationships c ON c.guardian_declaration_id=d.id AND c.carer_id=d.guardian_person_id AND c.patient_id=d.subject_person_id JOIN public.household_memberships m ON m.person_id=d.subject_person_id AND m.household_id=c.household_id WHERE d.guardian_person_id=$1 AND d.withdrawn_at IS NULL AND d.superseded_at IS NULL AND c.household_id=$2 AND c.active AND m.status='active' AND m.revoked_at IS NULL ORDER BY d.id FOR SHARE OF d",
        [owner.person_id.into(),tenant.scope().household_id.into()],
    )).await?;
    let ids = locked
        .iter()
        .map(|row| row.try_get::<i64>("", "id"))
        .collect::<Result<Vec<_>, _>>()?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    access::recheck(tenant).await?;
    let owner = account::Entity::find_by_id(tenant.scope().actor.account_id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    let time_zone =
        crate::models::identity::time_zone::preferred(&owner.preferences, tenant.settings())
            .map_err(|_| OperationError::Unavailable)?;
    let clock = tenant
        .transaction()
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT pg_catalog.clock_timestamp() AS now",
        ))
        .await?
        .ok_or(OperationError::Unavailable)?;
    let now: chrono::DateTime<Utc> = clock.try_get("", "now")?;
    let today = now.with_timezone(&time_zone).date_naive();
    let guardian = access::person_queries::account_person(tenant.scope().actor)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Forbidden)?;
    if guardian.id != tenant.membership().person_id
        || guardian.person_type != 0
        || !guardian.has_capacity
        || guardian
            .date_of_birth
            .is_some_and(|birth| super::people::age(birth, today) < 18)
    {
        return Ok(Vec::new());
    }
    let rows = tenant.transaction().query_all_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT d.subject_person_id,p.person_type,p.date_of_birth,p.has_capacity FROM public.guardian_declarations d JOIN public.carer_relationships c ON c.guardian_declaration_id=d.id AND c.carer_id=d.guardian_person_id AND c.patient_id=d.subject_person_id JOIN public.people p ON p.id=d.subject_person_id JOIN public.household_memberships m ON m.person_id=p.id AND m.household_id=c.household_id WHERE d.guardian_person_id=$1 AND d.withdrawn_at IS NULL AND d.superseded_at IS NULL AND c.household_id=$2 AND c.active AND m.status='active' AND m.revoked_at IS NULL AND d.id=ANY($3::bigint[]) ORDER BY d.id",
        [guardian.id.into(),tenant.scope().household_id.into(),ids.into()],
    )).await?;
    let mut subjects = Vec::new();
    for row in rows {
        let kind: i32 = row.try_get("", "person_type")?;
        let birth: Option<NaiveDate> = row.try_get("", "date_of_birth")?;
        let capacity: bool = row.try_get("", "has_capacity")?;
        if !capacity
            && birth.is_some_and(|birth| {
                let years = super::people::age(birth, today);
                years >= 0 && matches!((kind, years < 18), (1, true) | (2, false))
            })
        {
            subjects.push(row.try_get("", "subject_person_id")?);
        }
    }
    Ok(subjects)
}

pub(crate) async fn record_initial(
    tenant: &TenantTransaction,
    subject: &person::Model,
    _attestation: InitialGuardianAttestation,
    today: NaiveDate,
    provenance: Option<&CredentialProvenance>,
) -> Result<DeclarationId, OperationError> {
    let guardian = eligible_actor(tenant, today).await?;
    let row = tenant.transaction().query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO public.guardian_declarations(subject_person_id,guardian_person_id,origin_household_id,declared_by_account_id,statement_version,subject_name_at_declaration) VALUES($1,$2,$3,$4,1,$5) RETURNING id,declared_at",
        [subject.id.into(),guardian.id.into(),tenant.scope().household_id.into(),tenant.scope().actor.account_id.into(),subject.name.clone().into()],
    )).await?.ok_or(OperationError::Unavailable)?;
    let id: i64 = row.try_get("", "id")?;
    let declared_at: chrono::DateTime<Utc> = row.try_get("", "declared_at")?;
    let mut context = crate::models::audit_context::household(tenant, provenance);
    context["policy_class"] = json!("InitialGuardianDeclaration");
    context["policy_query"] = json!("create_new_dependent");
    let state = json!({"id":id,"subject_person_id":subject.id,"guardian_person_id":guardian.id,
        "origin_household_id":tenant.scope().household_id,"declared_by_account_id":tenant.scope().actor.account_id,
        "statement_version":1,"source":"initial_attestation","declared_at":declared_at,
        "withdrawn_at":null,"superseded_at":null,"revision":1});
    version::ActiveModel {
        item_type: Set("GuardianDeclaration".to_owned()),
        item_id: Set(id),
        event: Set("create".to_owned()),
        object: Set(None),
        object_changes: Set(Some(state.to_string())),
        whodunnit: Set(Some(tenant.scope().actor.account_id.to_string())),
        request_id: Set(Some(tenant.scope().request_id.clone())),
        household_id: Set(Some(tenant.scope().household_id)),
        actor_membership_id: Set(Some(tenant.membership().id)),
        audit_context: Set(context.clone()),
        created_at: Set(Some(Utc::now().naive_utc())),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await?;
    let now = Utc::now().naive_utc();
    security_audit_event::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        actor_account_id: Set(Some(tenant.scope().actor.account_id)),
        actor_membership_id: Set(Some(tenant.membership().id)),
        event_type: Set("guardian_declaration.created".to_owned()),
        request_id: Set(Some(tenant.scope().request_id.clone())),
        metadata: Set(state),
        audit_context: Set(context),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await?;
    Ok(DeclarationId(id))
}
