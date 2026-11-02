use loco_rs::model::TenantQueryExt;
mod legacy;
mod persistence;
mod reading;
pub(crate) use reading::period_values;
mod writing;
use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    care::administration,
    entities::{
        api_change_event, household, membership, pause_period, person, person_medication, schedule,
    },
    errors::OperationError,
    identity::credentials::CredentialProvenance,
};
use chrono::Utc;
pub use legacy::{
    DirectionInput, EmptyChangeInput, pause_assignment, pause_schedule, reorder_assignment,
    resume_assignment, resume_schedule,
};
pub use reading::{Pagination, list};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseTransaction, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use uuid::Uuid;
use writing::source_identity;
pub use writing::{authorize_create, authorize_resume, authorize_source, create, resume};

#[derive(Clone, Copy)]
pub enum PauseSourceKind {
    Schedule,
    Assignment,
}

impl PauseSourceKind {
    fn parse(value: &str) -> Result<Self, OperationError> {
        match value {
            "schedule" => Ok(Self::Schedule),
            "person_medication" => Ok(Self::Assignment),
            _ => Err(invalid("source_type", "is invalid")),
        }
    }

    fn kind(self) -> Kind {
        match self {
            Self::Schedule => Kind::Schedule,
            Self::Assignment => Kind::Assignment,
        }
    }

    pub fn name(self) -> &'static str {
        self.kind().name()
    }
}

#[derive(Clone, Copy)]
pub enum PauseReason {
    OutOfSupply,
    TemporarilyNotNeeded,
    ClinicianAdvice,
    SideEffects,
    Other,
}

impl PauseReason {
    fn parse(value: &str) -> Result<Self, OperationError> {
        match value {
            "out_of_supply" => Ok(Self::OutOfSupply),
            "temporarily_not_needed" => Ok(Self::TemporarilyNotNeeded),
            "clinician_advice" => Ok(Self::ClinicianAdvice),
            "side_effects" => Ok(Self::SideEffects),
            "other" => Ok(Self::Other),
            _ => Err(invalid("reason", "is invalid")),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::OutOfSupply => "out_of_supply",
            Self::TemporarilyNotNeeded => "temporarily_not_needed",
            Self::ClinicianAdvice => "clinician_advice",
            Self::SideEffects => "side_effects",
            Self::Other => "other",
        }
    }
}

pub struct PauseCreate {
    source_kind: PauseSourceKind,
    source_id: String,
    reason: PauseReason,
    note: Option<String>,
}

pub struct PauseSource {
    kind: PauseSourceKind,
    id: String,
}

pub struct PauseSourceAttempt(Result<PauseSource, OperationError>);

impl PauseSourceAttempt {
    pub fn from_fields(kind: Option<&Value>, id: Option<&Value>) -> Self {
        let source = (|| {
            let kind = kind
                .and_then(Value::as_str)
                .ok_or_else(|| invalid("source_type", "is invalid"))?;
            let kind = PauseSourceKind::parse(kind)?;
            let id = id
                .and_then(Value::as_str)
                .filter(|id| Uuid::parse_str(id).is_ok())
                .ok_or_else(|| invalid("source_id", "must be a portable identifier"))?;
            Ok(PauseSource {
                kind,
                id: id.to_owned(),
            })
        })();
        Self(source)
    }

    pub fn from_body(body: &Value) -> Self {
        Self(source_identity(body).map(|(kind, id)| PauseSource {
            kind: match kind {
                Kind::Schedule => PauseSourceKind::Schedule,
                Kind::Assignment => PauseSourceKind::Assignment,
            },
            id: id.to_owned(),
        }))
    }

    fn into_result(self) -> Result<PauseSource, OperationError> {
        self.0
    }
}

impl PauseCreate {
    pub fn new(
        kind: &str,
        id: &str,
        reason: &str,
        note: Option<String>,
    ) -> Result<Self, OperationError> {
        let source_kind = PauseSourceKind::parse(kind)?;
        Uuid::parse_str(id).map_err(|_| invalid("source_id", "must be a portable identifier"))?;
        let reason = PauseReason::parse(reason)?;
        Ok(Self {
            source_kind,
            source_id: id.to_owned(),
            reason,
            note,
        })
    }

    pub fn parse(body: &Value) -> Result<Self, OperationError> {
        let source = PauseSourceAttempt::from_body(body).into_result()?;
        Self::parse_with_source(&source, body)
    }

    pub fn parse_with_source(source: &PauseSource, body: &Value) -> Result<Self, OperationError> {
        let outer = body
            .as_object()
            .ok_or_else(|| invalid("medication_pause_period", "is invalid"))?;
        let attrs = outer
            .get("medication_pause_period")
            .and_then(Value::as_object)
            .ok_or_else(|| invalid("medication_pause_period", "is invalid"))?;
        if outer.len() != 1
            || attrs.keys().any(|key| {
                !matches!(
                    key.as_str(),
                    "source_type" | "source_id" | "reason" | "note"
                )
            })
        {
            return Err(invalid(
                "medication_pause_period",
                "contains an unsupported field",
            ));
        }
        Self::from_source_fields(source, attrs.get("reason"), attrs.get("note"))
    }

    pub fn from_source_fields(
        source: &PauseSource,
        reason: Option<&Value>,
        note: Option<&Value>,
    ) -> Result<Self, OperationError> {
        Self::from_fields(source.kind.name(), &source.id, reason, note)
    }

    pub fn from_fields(
        kind: &str,
        id: &str,
        reason: Option<&Value>,
        note: Option<&Value>,
    ) -> Result<Self, OperationError> {
        let reason = reason
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("reason", "is invalid"))?;
        let mut input = Self::new(kind, id, reason, None)?;
        let note = match note {
            None | Some(Value::Null) => None,
            Some(Value::String(note)) => Some(note.clone()),
            _ => return Err(invalid("note", "must be a string")),
        };
        input.note = note;
        Ok(input)
    }

    pub fn source_kind(&self) -> &'static str {
        self.source_kind.name()
    }
    pub fn reason(&self) -> &'static str {
        self.reason.name()
    }
}

pub struct PauseWritten {
    pub row: pause_period::Model,
    pub body: Value,
    pub etag: String,
}

pub struct ResumeInput {
    pub id: String,
}

impl ResumeInput {
    pub fn parse(id: &str, body: &Value) -> Result<Self, OperationError> {
        if body.as_object().is_none_or(|value| !value.is_empty()) {
            return Err(invalid("body", "must be empty"));
        }
        Ok(Self { id: id.to_owned() })
    }

    pub fn new(id: &str) -> Self {
        Self { id: id.to_owned() }
    }
}

#[cfg(test)]
mod typed_command_tests {
    use super::*;

    #[test]
    fn pause_create_parses_source_and_reason_without_a_json_write_command() {
        let command = PauseCreate::parse(&json!({"medication_pause_period": {
            "source_type": "schedule",
            "source_id": "00112233-4455-6677-8899-aabbccddeeff",
            "reason": "clinician_advice",
            "note": "Synthetic note"
        }}))
        .unwrap();
        assert_eq!(command.source_kind(), "schedule");
        assert_eq!(command.reason(), "clinician_advice");
    }

    #[test]
    fn invalid_pause_reason_precedes_invalid_note() {
        let error = PauseCreate::from_fields(
            "schedule",
            "00112233-4455-6677-8899-aabbccddeeff",
            Some(&json!("bad")),
            Some(&json!(42)),
        )
        .err()
        .expect("both fields are invalid");
        let OperationError::Validation { details } = error else {
            panic!("expected validation")
        };
        assert_eq!(details["errors"]["reason"], json!(["is invalid"]));
    }
}
type ApiError = OperationError;
struct AuthContext<'a> {
    tenant: &'a TenantTransaction,
    provenance: Option<&'a CredentialProvenance>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Schedule,
    Assignment,
}
impl Kind {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "schedule" => Some(Self::Schedule),
            "person_medication" => Some(Self::Assignment),
            _ => None,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Schedule => "schedule",
            Self::Assignment => "person_medication",
        }
    }
    fn record_type(self) -> &'static str {
        match self {
            Self::Schedule => "Schedule",
            Self::Assignment => "PersonMedication",
        }
    }
}
enum Source {
    Schedule(schedule::Model),
    Assignment(person_medication::Model),
}
impl Source {
    fn kind(&self) -> Kind {
        match self {
            Self::Schedule(_) => Kind::Schedule,
            Self::Assignment(_) => Kind::Assignment,
        }
    }
    fn id(&self) -> i64 {
        match self {
            Self::Schedule(row) => row.id,
            Self::Assignment(row) => row.id,
        }
    }
    fn person_id(&self) -> i64 {
        match self {
            Self::Schedule(row) => row.person_id,
            Self::Assignment(row) => row.person_id,
        }
    }
    fn portable_id(&self) -> &str {
        match self {
            Self::Schedule(row) => &row.portable_id,
            Self::Assignment(row) => &row.portable_id,
        }
    }
    fn active(&self) -> bool {
        match self {
            Self::Schedule(row) => row.active,
            Self::Assignment(row) => row.active,
        }
    }
    fn retired(&self) -> bool {
        match self {
            Self::Schedule(row) => row.retired_at.is_some(),
            Self::Assignment(row) => row.retired_at.is_some(),
        }
    }
}
pub(crate) fn invalid(field: &str, message: &str) -> OperationError {
    OperationError::Validation {
        details: json!({"errors":{field:[message]}}),
    }
}
fn database_error(_: sea_orm::DbErr) -> OperationError {
    OperationError::Unavailable
}
async fn lock(tenant: &TenantTransaction) -> Result<(), OperationError> {
    household::Entity::find_by_id(tenant.scope().household_id)
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    access::recheck(tenant).await
}
async fn find_source(
    tenant: &TenantTransaction,
    kind: Kind,
    id: &str,
    retired: bool,
) -> Result<Source, OperationError> {
    let row = match kind {
        Kind::Schedule => {
            let mut query = schedule::Entity::find()
                .in_tenant(tenant.scope().household_id)
                .filter(schedule::Column::PortableId.eq(id));
            if !retired {
                query = query.filter(schedule::Column::RetiredAt.is_null());
            }
            let initial = query
                .clone()
                .one(tenant.transaction())
                .await?
                .ok_or(OperationError::NotFound)?;
            super::person_schedules::projection::refresh_one(tenant, initial.id).await?;
            query
                .lock_exclusive()
                .one(tenant.transaction())
                .await?
                .map(Source::Schedule)
        }
        Kind::Assignment => {
            let mut query = person_medication::Entity::find()
                .in_tenant(tenant.scope().household_id)
                .filter(person_medication::Column::PortableId.eq(id));
            if !retired {
                query = query.filter(person_medication::Column::RetiredAt.is_null());
            }
            query
                .lock_exclusive()
                .one(tenant.transaction())
                .await?
                .map(Source::Assignment)
        }
    }
    .ok_or(OperationError::NotFound)?;
    if !access::can_access_person(tenant, row.person_id(), PersonAccess::View).await? {
        return Err(OperationError::NotFound);
    }
    Ok(row)
}
async fn source_for_period(
    tenant: &TenantTransaction,
    period: &pause_period::Model,
) -> Result<Source, OperationError> {
    let source = if let Some(id) = period.schedule_id {
        super::person_schedules::projection::refresh_one(tenant, id).await?;
        schedule::Entity::find_by_id(id)
            .in_tenant(tenant.scope().household_id)
            .lock_exclusive()
            .one(tenant.transaction())
            .await?
            .map(Source::Schedule)
    } else if let Some(id) = period.person_medication_id {
        person_medication::Entity::find_by_id(id)
            .in_tenant(tenant.scope().household_id)
            .lock_exclusive()
            .one(tenant.transaction())
            .await?
            .map(Source::Assignment)
    } else {
        None
    }
    .ok_or(OperationError::NotFound)?;
    if source.retired()
        || !access::can_access_person(tenant, source.person_id(), PersonAccess::View).await?
    {
        return Err(OperationError::NotFound);
    }
    Ok(source)
}
fn timestamp(value: chrono::NaiveDateTime) -> String {
    value
        .and_utc()
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
fn period_snapshot(row: &pause_period::Model) -> Value {
    json!({"household_id":row.household_id,"portable_id":row.portable_id,"schedule_id":row.schedule_id,"person_medication_id":row.person_medication_id,"reason":row.reason,"note":row.note,"legacy_context":row.legacy_context,"recorded_by_membership_id":row.recorded_by_membership_id,"resumed_by_membership_id":row.resumed_by_membership_id,"started_at":row.started_at.map(timestamp),"ended_at":row.ended_at.map(timestamp)})
}
async fn record_version(
    context: &AuthContext<'_>,
    kind: &str,
    id: i64,
    event: &str,
    before: Option<Value>,
    after: Option<Value>,
) -> Result<(), OperationError> {
    administration::persistence::record_version_as(
        context.tenant,
        kind,
        id,
        event,
        before,
        after.ok_or(OperationError::Unavailable)?,
        context.provenance,
    )
    .await
}
struct SyncRecord<'a> {
    record_type: &'a str,
    record_id: i64,
    portable_id: &'a str,
    action: &'a str,
    person_portable_id: Option<&'a str>,
}
async fn record_change(
    db: &DatabaseTransaction,
    context: &AuthContext<'_>,
    request_id: &str,
    row: SyncRecord<'_>,
) -> Result<(), OperationError> {
    let now = Utc::now().naive_utc();
    api_change_event::ActiveModel{household_id:Set(context.tenant.scope().household_id),household_membership_id:Set(Some(context.tenant.membership().id)),account_id:Set(Some(context.tenant.scope().actor.account_id)),action:Set(row.action.into()),record_type:Set(row.record_type.into()),record_id:Set(row.record_id),record_portable_id:Set(Some(row.portable_id.into())),request_id:Set(Some(request_id.into())),metadata:Set(json!({"record_type":row.record_type,"record_id":row.record_id,"portable_id":row.portable_id,"person_portable_id":row.person_portable_id})),occurred_at:Set(now),created_at:Set(now),updated_at:Set(now),..Default::default()}.insert(db).await?;
    Ok(())
}
