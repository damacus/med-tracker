use crate::models::{
    access::{self, TenantTransaction},
    entities::{membership, person_medication, schedule, shared_execution, sharing_approval},
    errors::OperationError,
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect,
    sea_query::{Expr, Func, Query},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum SourceKind {
    Schedule,
    #[serde(rename = "person_medication")]
    Assignment,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Source {
    kind: SourceKind,
    id: i64,
    portable_id: String,
}

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
pub(super) struct Epoch {
    sharing_approval_id: Option<i64>,
    subject_membership_id: i64,
    subject_permissions_version: i32,
    sources: BTreeSet<Source>,
}

impl Epoch {
    pub(super) fn identity(&self) -> Result<String, OperationError> {
        let bytes = serde_json::to_vec(self).map_err(|_| OperationError::Unavailable)?;
        Ok(hex::encode(Sha256::digest(bytes)))
    }

    pub(super) fn metadata(&self, account: i64) -> Result<Value, OperationError> {
        let mut value = serde_json::to_value(self).map_err(|_| OperationError::Unavailable)?;
        value["account_id"] = json!(account);
        value["delivery_status"] = json!("pending");
        Ok(value)
    }

    pub(super) fn matches(&self, metadata: &Value) -> bool {
        if [
            "sharing_approval_id",
            "subject_membership_id",
            "subject_permissions_version",
            "sources",
        ]
        .iter()
        .any(|field| metadata.get(field).is_none())
        {
            return false;
        }
        serde_json::from_value::<Self>(metadata.clone()).is_ok_and(|stored| stored == *self)
    }
}

pub(super) async fn capture(
    tenant: &TenantTransaction,
    subject: i64,
    keys: Vec<(&'static str, i64)>,
) -> Result<Option<Epoch>, OperationError> {
    access::recheck(tenant).await?;
    let db = tenant.transaction();
    let home = tenant.scope().household_id;
    let Some(member) = membership::Entity::find()
        .filter(membership::Column::HouseholdId.eq(home))
        .filter(membership::Column::PersonId.eq(subject))
        .filter(membership::Column::Status.eq("active"))
        .filter(membership::Column::RevokedAt.is_null())
        .one(db)
        .await?
    else {
        return Ok(None);
    };
    let approvals = sharing_approval::Entity::find()
        .filter(sharing_approval::Column::HouseholdId.eq(home))
        .filter(sharing_approval::Column::PersonId.eq(subject))
        .all(db)
        .await?;
    let approval = approvals
        .iter()
        .find(|approval| approval.ended_at.is_none())
        .map(|approval| approval.id);
    if approval.is_none() && !approvals.is_empty() {
        return Ok(None);
    }
    if approval.is_some() {
        let query = Query::select()
            .expr_as(
                Func::cust("med_tracker.sharing_current_member").arg(Expr::val(subject)),
                "current",
            )
            .to_owned();
        let current = db
            .query_one_raw(db.get_database_backend().build(&query))
            .await?
            .ok_or(OperationError::Unavailable)?
            .try_get::<bool>("", "current")?;
        if !current {
            return Ok(None);
        }
    }
    let mut sources = BTreeSet::new();
    for (kind, id) in keys {
        let source = match kind {
            "schedule" => {
                let Some(row) = schedule::Entity::find_by_id(id)
                    .filter(schedule::Column::HouseholdId.eq(home))
                    .filter(schedule::Column::PersonId.eq(subject))
                    .filter(schedule::Column::ExecutionEndedAt.is_null())
                    .filter(schedule::Column::RetiredAt.is_null())
                    .filter(schedule::Column::Active.eq(true))
                    .one(db)
                    .await?
                else {
                    return Ok(None);
                };
                let link = shared_execution::Entity::find_by_id(id).one(db).await?;
                match (approval, row.person_schedule_id.as_ref(), link.as_ref()) {
                    (Some(approval), Some(plan), Some(link))
                        if link.approval_id == Some(approval)
                            && link.person_schedule_id == *plan
                            && link.person_id == subject
                            && link.household_id == home
                            && link.portable_id == row.portable_id => {}
                    (None, Some(plan), Some(link))
                        if row.canonical_origin
                            && link.approval_id.is_none()
                            && link.person_schedule_id == *plan
                            && link.person_id == subject
                            && link.household_id == home
                            && link.portable_id == row.portable_id => {}
                    (None, None, None) => {}
                    _ => return Ok(None),
                }
                Source {
                    kind: SourceKind::Schedule,
                    id,
                    portable_id: row.portable_id,
                }
            }
            "person_medication" => {
                let Some(row) = person_medication::Entity::find_by_id(id)
                    .filter(person_medication::Column::HouseholdId.eq(home))
                    .filter(person_medication::Column::PersonId.eq(subject))
                    .filter(person_medication::Column::RetiredAt.is_null())
                    .filter(person_medication::Column::Active.eq(true))
                    .one(db)
                    .await?
                else {
                    return Ok(None);
                };
                Source {
                    kind: SourceKind::Assignment,
                    id,
                    portable_id: row.portable_id,
                }
            }
            _ => return Err(OperationError::Unavailable),
        };
        sources.insert(source);
    }
    if sources.is_empty() {
        return Ok(None);
    }
    Ok(Some(Epoch {
        sharing_approval_id: approval,
        subject_membership_id: member.id,
        subject_permissions_version: member.permissions_version,
        sources,
    }))
}

pub(super) async fn for_medication(
    tenant: &TenantTransaction,
    subject: i64,
    medication: i64,
) -> Result<Option<Epoch>, OperationError> {
    let schedules: Vec<i64> = schedule::Entity::find()
        .select_only()
        .column(schedule::Column::Id)
        .filter(schedule::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(schedule::Column::PersonId.eq(subject))
        .filter(schedule::Column::MedicationId.eq(medication))
        .filter(schedule::Column::ExecutionEndedAt.is_null())
        .filter(schedule::Column::RetiredAt.is_null())
        .filter(schedule::Column::Active.eq(true))
        .into_tuple()
        .all(tenant.transaction())
        .await?;
    let assignments: Vec<i64> = person_medication::Entity::find()
        .select_only()
        .column(person_medication::Column::Id)
        .filter(person_medication::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(person_medication::Column::PersonId.eq(subject))
        .filter(person_medication::Column::MedicationId.eq(medication))
        .filter(person_medication::Column::RetiredAt.is_null())
        .filter(person_medication::Column::Active.eq(true))
        .into_tuple()
        .all(tenant.transaction())
        .await?;
    capture(
        tenant,
        subject,
        schedules
            .into_iter()
            .map(|id| ("schedule", id))
            .chain(assignments.into_iter().map(|id| ("person_medication", id)))
            .collect(),
    )
    .await
}
