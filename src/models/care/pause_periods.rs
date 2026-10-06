mod legacy;
mod persistence;
mod reading;
pub(crate) use reading::period_values;
mod writing;
use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    care::{administration, doses::CredentialProvenance},
    entities::{
        api_change_event, household, membership, pause_period, person, person_medication, schedule,
    },
    errors::OperationError,
};
use chrono::Utc;
pub use legacy::{pause_assignment, pause_schedule, resume_assignment, resume_schedule};
pub use reading::{Pagination, list};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseTransaction, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use uuid::Uuid;
pub use writing::{authorize_create, authorize_resume, create, resume};
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
fn invalid(field: &str, message: &str) -> OperationError {
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
                .filter(schedule::Column::HouseholdId.eq(tenant.scope().household_id))
                .filter(schedule::Column::PortableId.eq(id));
            if !retired {
                query = query.filter(schedule::Column::RetiredAt.is_null());
            }
            query
                .lock_exclusive()
                .one(tenant.transaction())
                .await?
                .map(Source::Schedule)
        }
        Kind::Assignment => {
            let mut query = person_medication::Entity::find()
                .filter(person_medication::Column::HouseholdId.eq(tenant.scope().household_id))
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
        schedule::Entity::find_by_id(id)
            .filter(schedule::Column::HouseholdId.eq(tenant.scope().household_id))
            .lock_exclusive()
            .one(tenant.transaction())
            .await?
            .map(Source::Schedule)
    } else if let Some(id) = period.person_medication_id {
        person_medication::Entity::find_by_id(id)
            .filter(person_medication::Column::HouseholdId.eq(tenant.scope().household_id))
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
