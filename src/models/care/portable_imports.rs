mod access;
mod contracts;
mod declarations;
mod identity;
mod links;
mod mapping;
mod pauses;
mod preflight;
mod validation;
mod writing;

pub(crate) use contracts::ImportPlan;
pub(crate) use declarations::{InitialDeclarations, PreparedDeclarations};

pub(crate) use access::authorized;
pub(crate) use identity::ExistingIndex;
pub(crate) use preflight::preflight;
pub(crate) use writing::write_records;

use crate::models::{
    access::TenantTransaction, care::administration, care::doses::CredentialProvenance,
    entities::api_change_event, errors::OperationError,
};
use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, Utc};
use sea_orm::prelude::Decimal;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseTransaction, DbBackend, Set, Statement};
use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

pub(super) struct SyncRecord<'a> {
    pub record_type: &'a str,
    pub record_id: i64,
    pub portable_id: &'a str,
    pub action: &'a str,
    pub person_portable_id: Option<&'a str>,
}

pub(super) async fn record_change(
    db: &DatabaseTransaction,
    tenant: &TenantTransaction,
    request_id: &str,
    row: SyncRecord<'_>,
) -> Result<(), OperationError> {
    let now = Utc::now().naive_utc();
    api_change_event::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        household_membership_id: Set(Some(tenant.membership().id)),
        account_id: Set(Some(tenant.scope().actor.account_id)),
        action: Set(row.action.into()),
        record_type: Set(row.record_type.into()),
        record_id: Set(row.record_id),
        record_portable_id: Set(Some(row.portable_id.into())),
        request_id: Set(Some(request_id.into())),
        metadata: Set(json!({"record_type":row.record_type,"record_id":row.record_id,"portable_id":row.portable_id,"person_portable_id":row.person_portable_id})),
        occurred_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}

use contracts::{BASE_TYPES, V2_TYPES};
use identity::table;
use links::write_health_links;
use mapping::{mapped_row, record_person, text};
use pauses::{reconcile_pauses, resume_pause, verify_pause};
use validation::{decimal, validate_row};
use writing::verify_immutable;
