use super::*;
use crate::models::entities::{
    adoption_fact, medication_take, shared_administration, shared_schedule, version,
};
use sea_orm::{
    ConnectionTrait, QueryOrder,
    sea_query::{Alias, Expr, ExprTrait, Func, Query, SimpleExpr},
};
use sha2::{Digest, Sha256};

pub(super) struct CapturedHistory(Vec<adoption_fact::Model>);

fn unavailable() -> OperationError {
    crate::models::profile::validation("history", "Original administration evidence is unavailable")
}

#[derive(serde::Deserialize)]
struct AuditedTake {
    id: i64,
    portable_id: String,
    client_uuid: Option<String>,
    household_id: i64,
    schedule_id: Option<i64>,
    person_medication_id: Option<i64>,
    taken_from_medication_id: Option<i64>,
    taken_from_location_id: Option<i64>,
    dose_amount: Option<sea_orm::prelude::Decimal>,
    dose_unit: Option<String>,
    taken_at: Option<String>,
    created_at: String,
    updated_at: String,
}

fn same_time(encoded: &str, actual: chrono::NaiveDateTime) -> bool {
    use chrono::Timelike;
    let decoded = chrono::DateTime::parse_from_rfc3339(encoded)
        .map(|value| value.naive_utc())
        .or_else(|_| encoded.parse::<chrono::NaiveDateTime>());
    decoded.is_ok_and(|value| {
        value == actual || (!encoded.contains('.') && actual.with_nanosecond(0) == Some(value))
    })
}

fn matches_audit(
    take: &medication_take::Model,
    original: &version::Model,
) -> Result<bool, OperationError> {
    let payload = if let Some(snapshot) = original.object.as_ref() {
        serde_json::from_str(snapshot).map_err(|_| unavailable())?
    } else {
        let changes: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(original.object_changes.as_deref().ok_or_else(unavailable)?)
                .map_err(|_| unavailable())?;
        let fields = changes
            .into_iter()
            .map(|(key, value)| {
                let values = value
                    .as_array()
                    .filter(|values| {
                        values.len() == 2 && values.first() == Some(&serde_json::Value::Null)
                    })
                    .ok_or_else(unavailable)?;
                Ok((key, values.get(1).cloned().ok_or_else(unavailable)?))
            })
            .collect::<Result<serde_json::Map<_, _>, OperationError>>()?;
        serde_json::Value::Object(fields)
    };
    let fact: AuditedTake = serde_json::from_value(payload).map_err(|_| unavailable())?;
    Ok(original.household_id == Some(take.household_id)
        && fact.id == take.id
        && fact.portable_id == take.portable_id
        && fact.client_uuid == take.client_uuid
        && fact.household_id == take.household_id
        && fact.schedule_id == take.schedule_id
        && fact.person_medication_id == take.person_medication_id
        && fact.taken_from_medication_id == take.taken_from_medication_id
        && fact.taken_from_location_id == take.taken_from_location_id
        && fact.dose_amount == take.dose_amount
        && fact.dose_unit == take.dose_unit
        && fact
            .taken_at
            .as_deref()
            .zip(take.taken_at)
            .is_some_and(|(text, time)| same_time(text, time))
        && same_time(&fact.created_at, take.created_at)
        && same_time(&fact.updated_at, take.updated_at))
}

async fn fact_hash(
    db: &DatabaseTransaction,
    fact: &adoption_fact::Model,
) -> Result<Vec<u8>, OperationError> {
    let text = |value: Option<String>| Expr::val(value).cast_as(Alias::new("text"));
    let integer = |value: Option<i64>| Expr::val(value).cast_as(Alias::new("bigint"));
    let timestamp = |value| {
        Expr::expr(Func::cust("to_char").args([
            Expr::val(value).cast_as(Alias::new("timestamp")),
            Expr::val("YYYY-MM-DD\"T\"HH24:MI:SS.US"),
        ]))
    };
    let values: Vec<SimpleExpr> = vec![
        text(Some(
            if fact.person_schedule_id.is_some() {
                "live-adoption-v1"
            } else {
                "frozen-departure-v1"
            }
            .into(),
        )),
        text(fact.person_schedule_id.clone()),
        integer(Some(fact.source_schedule_id)),
        text(Some(fact.source_portable_id.clone())),
        integer(Some(fact.person_id)),
        integer(Some(fact.household_id)),
        integer(Some(fact.medication_take_id)),
        text(Some(fact.take_portable_id.clone())),
        integer(Some(fact.original_version_id)),
        text(fact.client_uuid.clone()),
        timestamp(fact.taken_at),
        Expr::val(fact.dose_amount).cast_as(Alias::new("numeric")),
        text(fact.dose_unit.clone()),
        integer(fact.taken_from_medication_id),
        integer(fact.taken_from_location_id),
        timestamp(fact.take_created_at),
        timestamp(fact.take_updated_at),
    ];
    let encoding =
        Expr::expr(Func::cust("jsonb_build_array").args(values)).cast_as(Alias::new("text"));
    let query = Query::select()
        .expr_as(
            Func::cust("convert_to").args([encoding, Expr::val("UTF8")]),
            Alias::new("fact_bytes"),
        )
        .to_owned();
    let encoded = db
        .query_one_raw(db.get_database_backend().build(&query))
        .await?
        .ok_or(OperationError::Unavailable)?
        .try_get::<Vec<u8>>("", "fact_bytes")?;
    Ok(Sha256::digest(encoded).to_vec())
}

pub(super) async fn capture(
    tenant: &TenantTransaction,
    source: &schedule::Model,
    plan: &person_schedule::Model,
    provenance: Option<&crate::models::identity::credentials::CredentialProvenance>,
) -> Result<CapturedHistory, OperationError> {
    if source.person_schedule_id.is_some() {
        return Ok(CapturedHistory(Vec::new()));
    }
    let db = tenant.transaction();
    crate::models::access::require_person_access(
        tenant,
        source.person_id,
        crate::models::access::PersonAccess::Manage,
    )
    .await?;
    let declaration = if tenant.membership().person_id == source.person_id {
        None
    } else {
        Some(
            sharing::guardian(
                db,
                crate::models::ids::PersonId::from_database(source.person_id),
            )
            .await?
            .ok_or(OperationError::Forbidden)?,
        )
    };
    let takes = medication_take::Entity::find()
        .filter(medication_take::Column::HouseholdId.eq(source.household_id))
        .filter(medication_take::Column::ScheduleId.eq(source.id))
        .order_by_asc(medication_take::Column::Id)
        .lock_shared()
        .all(db)
        .await?;
    let mut captured = Vec::with_capacity(takes.len());
    for take in takes {
        let original = version::Entity::find()
            .filter(version::Column::ItemType.eq("MedicationTake"))
            .filter(version::Column::ItemId.eq(take.id))
            .filter(version::Column::Event.eq("create"))
            .order_by_asc(version::Column::Id)
            .one(db)
            .await?
            .ok_or_else(unavailable)?;
        if !matches_audit(&take, &original)? {
            return Err(unavailable());
        }
        let mut fact = adoption_fact::Model {
            medication_take_id: take.id,
            person_schedule_id: Some(plan.id.clone()),
            source_schedule_id: source.id,
            source_portable_id: source.portable_id.clone(),
            person_id: source.person_id,
            household_id: source.household_id,
            take_portable_id: take.portable_id,
            original_version_id: original.id,
            client_uuid: take.client_uuid,
            taken_at: take.taken_at.ok_or_else(unavailable)?,
            dose_amount: take
                .dose_amount
                .filter(|amount| *amount > sea_orm::prelude::Decimal::ZERO)
                .ok_or_else(unavailable)?,
            dose_unit: take.dose_unit,
            taken_from_medication_id: take.taken_from_medication_id,
            taken_from_location_id: take.taken_from_location_id,
            take_created_at: take.created_at,
            take_updated_at: take.updated_at,
            fact_hash: Vec::new(),
            captured_by_account_id: tenant.scope().actor.account_id,
            captured_by_person_id: tenant.membership().person_id,
            captured_by_membership_id: tenant.membership().id,
            guardian_declaration_id: declaration,
            request_id: tenant.scope().request_id.clone(),
            authentication_method: provenance.map(|value| value.method.label().into()),
            credential_reference: provenance.map(|value| value.reference.clone()),
            captured_at: Utc::now().naive_utc(),
        };
        fact.fact_hash = fact_hash(db, &fact).await?;
        let fact = fact.into_active_model().insert(db).await?;
        captured.push(fact);
    }
    Ok(CapturedHistory(captured))
}

/// Frozen-departure capture: runs inside `legacy::freeze` while the departing
/// household's management authority and row locks still exist. Takes whose
/// original evidence cannot be verified are skipped rather than blocking a
/// permitted operational departure; database errors still propagate.
pub(super) async fn capture_frozen(
    tenant: &TenantTransaction,
    source: &schedule::Model,
    provenance: Option<&crate::models::identity::credentials::CredentialProvenance>,
) -> Result<(), OperationError> {
    let db = tenant.transaction();
    let takes = medication_take::Entity::find()
        .filter(medication_take::Column::HouseholdId.eq(source.household_id))
        .filter(medication_take::Column::ScheduleId.eq(source.id))
        .order_by_asc(medication_take::Column::Id)
        .lock_shared()
        .all(db)
        .await?;
    for take in takes {
        let Some(original) = version::Entity::find()
            .filter(version::Column::ItemType.eq("MedicationTake"))
            .filter(version::Column::ItemId.eq(take.id))
            .filter(version::Column::Event.eq("create"))
            .order_by_asc(version::Column::Id)
            .one(db)
            .await?
        else {
            continue;
        };
        if !matches_audit(&take, &original).unwrap_or(false) {
            continue;
        }
        let (Some(taken_at), Some(dose_amount)) = (
            take.taken_at,
            take.dose_amount
                .filter(|amount| *amount > sea_orm::prelude::Decimal::ZERO),
        ) else {
            continue;
        };
        let mut fact = adoption_fact::Model {
            medication_take_id: take.id,
            person_schedule_id: None,
            source_schedule_id: source.id,
            source_portable_id: source.portable_id.clone(),
            person_id: source.person_id,
            household_id: source.household_id,
            take_portable_id: take.portable_id,
            original_version_id: original.id,
            client_uuid: take.client_uuid,
            taken_at,
            dose_amount,
            dose_unit: take.dose_unit,
            taken_from_medication_id: take.taken_from_medication_id,
            taken_from_location_id: take.taken_from_location_id,
            take_created_at: take.created_at,
            take_updated_at: take.updated_at,
            fact_hash: Vec::new(),
            captured_by_account_id: tenant.scope().actor.account_id,
            captured_by_person_id: tenant.membership().person_id,
            captured_by_membership_id: tenant.membership().id,
            guardian_declaration_id: None,
            request_id: tenant.scope().request_id.clone(),
            authentication_method: provenance.map(|value| value.method.label().into()),
            credential_reference: provenance.map(|value| value.reference.clone()),
            captured_at: Utc::now().naive_utc(),
        };
        fact.fact_hash = fact_hash(db, &fact).await?;
        fact.into_active_model().insert(db).await?;
    }
    Ok(())
}

/// Restore-time capture for a take written onto an already-linked canonical
/// source: mints the immutable fact bound to the source's current plan link
/// and seeds the minimal safety index row. Takes on unlinked or ended sources
/// return `false` untouched, and a take that already carries a fact is
/// skipped so backup replay stays idempotent. The mint itself enforces the
/// same household actor, clinical-authority, captured-actor and
/// guardian-declaration predicates as adoption capture, and the source row is
/// locked so revocation cannot slip past the check.
pub(crate) async fn index_restored(
    tenant: &TenantTransaction,
    take: &medication_take::Model,
    provenance: Option<&crate::models::identity::credentials::CredentialProvenance>,
) -> Result<bool, OperationError> {
    let Some(schedule_id) = take.schedule_id else {
        return Ok(false);
    };
    if take.person_medication_id.is_some() || take.household_id != tenant.scope().household_id {
        return Ok(false);
    }
    let db = tenant.transaction();
    let Some(source) = schedule::Entity::find_by_id(schedule_id)
        .filter(schedule::Column::HouseholdId.eq(take.household_id))
        .filter(schedule::Column::ExecutionEndedAt.is_null())
        .filter(schedule::Column::PersonScheduleId.is_not_null())
        .filter(schedule::Column::CanonicalOrigin.eq(true))
        .lock_shared()
        .one(db)
        .await?
    else {
        return Ok(false);
    };
    let Some(plan) = source.person_schedule_id.clone() else {
        return Ok(false);
    };
    if adoption_fact::Entity::find_by_id(take.id)
        .one(db)
        .await?
        .is_some()
    {
        return Ok(false);
    }
    crate::models::access::require_person_access(
        tenant,
        source.person_id,
        crate::models::access::PersonAccess::Manage,
    )
    .await?;
    let declaration = if tenant.membership().person_id == source.person_id {
        None
    } else {
        Some(
            sharing::guardian(
                db,
                crate::models::ids::PersonId::from_database(source.person_id),
            )
            .await?
            .ok_or(OperationError::Forbidden)?,
        )
    };
    let original = version::Entity::find()
        .filter(version::Column::ItemType.eq("MedicationTake"))
        .filter(version::Column::ItemId.eq(take.id))
        .filter(version::Column::Event.eq("create"))
        .order_by_asc(version::Column::Id)
        .one(db)
        .await?
        .ok_or_else(unavailable)?;
    if !matches_audit(take, &original)? {
        return Err(unavailable());
    }
    let mut fact = adoption_fact::Model {
        medication_take_id: take.id,
        person_schedule_id: Some(plan.clone()),
        source_schedule_id: source.id,
        source_portable_id: source.portable_id.clone(),
        person_id: source.person_id,
        household_id: source.household_id,
        take_portable_id: take.portable_id.clone(),
        original_version_id: original.id,
        client_uuid: take.client_uuid.clone(),
        taken_at: take.taken_at.ok_or_else(unavailable)?,
        dose_amount: take
            .dose_amount
            .filter(|amount| *amount > sea_orm::prelude::Decimal::ZERO)
            .ok_or_else(unavailable)?,
        dose_unit: take.dose_unit.clone(),
        taken_from_medication_id: take.taken_from_medication_id,
        taken_from_location_id: take.taken_from_location_id,
        take_created_at: take.created_at,
        take_updated_at: take.updated_at,
        fact_hash: Vec::new(),
        captured_by_account_id: tenant.scope().actor.account_id,
        captured_by_person_id: tenant.membership().person_id,
        captured_by_membership_id: tenant.membership().id,
        guardian_declaration_id: declaration,
        request_id: tenant.scope().request_id.clone(),
        authentication_method: provenance.map(|value| value.method.label().into()),
        credential_reference: provenance.map(|value| value.reference.clone()),
        captured_at: Utc::now().naive_utc(),
    };
    fact.fact_hash = fact_hash(db, &fact).await?;
    // Only this restore path may mint evidence on an already-linked source:
    // the insert policy and guard require the restoring take marker, so a
    // linked source without it keeps the prospective-adoption-only binding
    // and real fresh doses cannot gain late evidence.
    db.query_one_raw(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        "SELECT set_config('med_tracker.restoring_take_id', $1, true)",
        [take.id.to_string().into()],
    ))
    .await?;
    let minted = fact.into_active_model().insert(db).await;
    db.query_one_raw(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        "SELECT set_config('med_tracker.restoring_take_id', $1, true)",
        [String::new().into()],
    ))
    .await?;
    let fact = minted?;
    CapturedHistory::bind(vec![fact]).seed(db, &plan).await?;
    Ok(true)
}

impl CapturedHistory {
    pub(super) fn bind(facts: Vec<adoption_fact::Model>) -> Self {
        Self(facts)
    }

    /// Seeds the minimal safety index rows bound to `plan` and raises
    /// `latest_taken_at`. Live facts carry the same plan id; frozen facts are
    /// bound by the immutable parent's `adopted_person_schedule_id`.
    pub(super) async fn seed(
        self,
        db: &DatabaseTransaction,
        plan: &str,
    ) -> Result<(), OperationError> {
        for fact in &self.0 {
            shared_administration::Entity::insert(shared_administration::ActiveModel {
                person_schedule_id: Set(plan.to_owned()),
                household_id: Set(fact.household_id),
                medication_take_id: Set(fact.medication_take_id),
                taken_at: Set(fact.taken_at),
                occurrence_starts_on: Set(None),
                occurrence_position: Set(None),
                ..Default::default()
            })
            .exec(db)
            .await?;
        }
        if let Some(latest) = self.0.iter().map(|fact| fact.taken_at).max() {
            shared_schedule::Entity::update_many()
                .col_expr(shared_schedule::Column::LatestTakenAt, Expr::val(latest))
                .filter(shared_schedule::Column::Id.eq(plan))
                .filter(
                    sea_orm::Condition::any()
                        .add(shared_schedule::Column::LatestTakenAt.is_null())
                        .add(shared_schedule::Column::LatestTakenAt.lt(latest)),
                )
                .exec(db)
                .await?;
        }
        Ok(())
    }
}
