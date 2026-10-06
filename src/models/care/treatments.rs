mod input;
pub mod lifecycle;
mod projection;
mod source_stock;
use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    care::{administration, doses::CredentialProvenance, medications},
    entities::{api_change_event, dosage, household, medication, person, schedule},
    errors::OperationError,
};
use chrono::{NaiveDate, Utc};
use sea_orm::prelude::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, QuerySelect, Set,
};
use serde_json::{Map, Value, json};
use std::str::FromStr;
use uuid::Uuid;
type ApiError = OperationError;
type AuthContext = TenantTransaction;
async fn find_person(
    db: &DatabaseTransaction,
    context: &TenantTransaction,
    id: &str,
) -> Result<Option<person::Model>, OperationError> {
    let query =
        person::Entity::find().filter(person::Column::HouseholdId.eq(context.scope().household_id));
    let query = if let Ok(id) = id.parse::<i64>() {
        query.filter(person::Column::Id.eq(id))
    } else {
        query.filter(person::Column::PortableId.eq(id))
    };
    Ok(query.one(db).await?)
}
async fn person_access(
    _: &DatabaseTransaction,
    context: &TenantTransaction,
    id: i64,
    manage: bool,
) -> Result<bool, OperationError> {
    match access::require_person_access(
        context,
        id,
        if manage {
            PersonAccess::Manage
        } else {
            PersonAccess::View
        },
    )
    .await
    {
        Ok(()) => Ok(true),
        Err(OperationError::Forbidden | OperationError::NotFound) => Ok(false),
        Err(error) => Err(error),
    }
}
async fn visible_medication(
    db: &DatabaseTransaction,
    context: &TenantTransaction,
    id: &str,
) -> Result<Option<medication::Model>, OperationError> {
    let query = access::medication_scope(context);
    let query = if let Ok(id) = id.parse::<i64>() {
        query.filter(medication::Column::Id.eq(id))
    } else {
        query.filter(medication::Column::PortableId.eq(id))
    };
    Ok(query.one(db).await?)
}
async fn find_dosage(
    db: &DatabaseTransaction,
    context: &TenantTransaction,
    id: &str,
) -> Result<Option<dosage::Model>, OperationError> {
    let query =
        dosage::Entity::find().filter(dosage::Column::HouseholdId.eq(context.scope().household_id));
    let query = if let Ok(id) = id.parse::<i64>() {
        query.filter(dosage::Column::Id.eq(id))
    } else {
        query.filter(dosage::Column::PortableId.eq(id))
    };
    if let Some(row) = query.one(db).await?
        && visible_medication(db, context, &row.medication_id.to_string())
            .await?
            .is_some()
    {
        return Ok(Some(row));
    }
    Ok(None)
}
fn failure(error: InputFailure) -> OperationError {
    match error {
        InputFailure::NotFound => OperationError::NotFound,
        InputFailure::Malformed => OperationError::Validation {
            details: json!({"errors":{"schedule":["is invalid"]}}),
        },
        InputFailure::Invalid(field, message) => OperationError::Validation {
            details: json!({"errors":{field:[message]}}),
        },
        InputFailure::InvalidFields(errors) => OperationError::Validation {
            details: json!({"errors":errors}),
        },
    }
}
pub async fn authorize_person(
    tenant: &TenantTransaction,
    body: &Value,
) -> Result<(), OperationError> {
    household::Entity::find_by_id(tenant.scope().household_id)
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    access::recheck(tenant).await?;
    let attributes = input::attributes(body).map_err(failure)?;
    let id = attributes
        .get("person_id")
        .ok_or_else(|| failure(InputFailure::Invalid("person_id", "can't be blank")))?;
    let id = input::identifier(id, "person_id").map_err(failure)?;
    let row = find_person(tenant.transaction(), tenant, id)
        .await?
        .ok_or(OperationError::NotFound)?;
    if !person_access(tenant.transaction(), tenant, row.id, false).await? {
        return Err(OperationError::NotFound);
    }
    access::require_person_access(tenant, row.id, PersonAccess::Manage).await
}
pub async fn create(
    tenant: &TenantTransaction,
    body: &Value,
    provenance: Option<&CredentialProvenance>,
) -> Result<(Value, String), OperationError> {
    authorize_person(tenant, body).await?;
    let attributes = input::attributes(body).map_err(failure)?;
    let mut fields = ScheduleFields::new();
    input::apply_attributes(tenant.transaction(), tenant, attributes, &mut fields, None)
        .await?
        .map_err(failure)?;
    fields.validate().map_err(failure)?;
    let now = Utc::now().naive_utc();
    let mut active = schedule::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        portable_id: Set(Uuid::new_v4().to_string()),
        active: Set(true),
        retired_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    fields.assign(&mut active);
    let row = active.insert(tenant.transaction()).await?;
    persist(tenant, &row, None, "create", provenance).await?;
    lifecycle::project(tenant, row).await
}
async fn persist(
    tenant: &TenantTransaction,
    row: &schedule::Model,
    before: Option<Value>,
    action: &str,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    let snapshot = representation(row).0["data"].clone();
    administration::persistence::record_version_as(
        tenant, "Schedule", row.id, action, before, snapshot, provenance,
    )
    .await?;
    let now = Utc::now().naive_utc();
    api_change_event::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        household_membership_id: Set(Some(tenant.membership().id)),
        account_id: Set(Some(tenant.scope().actor.account_id)),
        action: Set(action.into()),
        record_type: Set("Schedule".into()),
        record_id: Set(row.id),
        record_portable_id: Set(Some(row.portable_id.clone())),
        request_id: Set(Some(tenant.scope().request_id.clone())),
        metadata: Set(
            json!({"record_type":"Schedule","record_id":row.id,"portable_id":row.portable_id}),
        ),
        occurred_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await?;
    Ok(())
}
fn representation(row: &schedule::Model) -> (Value, String) {
    let body = json!({"data":{"id":row.id,"portable_id":row.portable_id,"person_id":row.person_id,"medication_id":row.medication_id,"source_dosage_option_id":row.source_dosage_option_id,"active":row.active,"dose_amount":row.dose_amount.map(|value|medications::decimal_string(value.to_string())),"dose_unit":row.dose_unit,"frequency":row.frequency,"start_date":row.start_date,"end_date":row.end_date,"notes":row.notes,"max_daily_doses":row.max_daily_doses,"min_hours_between_doses":row.min_hours_between_doses.map(|value|format!("{value}.0")),"dose_cycle":row.dose_cycle.map(|value|match value{1=>"weekly",2=>"monthly",_=>"daily"}),"schedule_type":match row.schedule_type{1=>"multiple_daily",2=>"weekly",3=>"specific_dates",4=>"prn",5=>"tapering",6=>"every_other_day",_=>"daily"},"schedule_config":row.schedule_config,"updated_at":row.updated_at}});
    use sha2::{Digest, Sha256};
    let mut sorted = body.clone();
    sorted.sort_all_objects();
    let etag = format!(
        "\"{}\"",
        hex::encode(Sha256::digest(
            serde_json::to_vec(&sorted).expect("JSON must serialize")
        ))
    );
    (body, etag)
}
fn database_error(_: sea_orm::DbErr) -> OperationError {
    OperationError::Unavailable
}
fn valid_identifier(id: &str) -> bool {
    id.parse::<i64>().is_ok_and(|id| id > 0) || Uuid::parse_str(id).is_ok()
}
fn parse_decimal(value: &Value) -> Option<Decimal> {
    Decimal::from_str(value.as_str()?).ok()
}
fn storage_decimal(value: Decimal, integer: u32, fraction: u32) -> Option<Decimal> {
    let limit = Decimal::from(10_i64.pow(integer));
    (value.abs() < limit && value.normalize().scale() <= fraction).then_some(value)
}
const UNITS: &[&str] = &[
    "tablet", "capsule", "gummy", "mg", "ml", "g", "mcg", "IU", "spray", "drop", "sachet", "pad",
];
const FIELDS: &[&str] = &[
    "person_id",
    "medication_id",
    "source_dosage_option_id",
    "dose_amount",
    "dose_unit",
    "frequency",
    "start_date",
    "end_date",
    "notes",
    "max_daily_doses",
    "min_hours_between_doses",
    "dose_cycle",
    "schedule_type",
    "schedule_config",
];

enum InputFailure {
    Malformed,
    NotFound,
    Invalid(&'static str, &'static str),
    InvalidFields(Value),
}

#[derive(PartialEq)]
struct ScheduleFields {
    person_id: Option<i64>,
    medication_id: Option<i64>,
    source_dosage_option_id: Option<i64>,
    dose_amount: Option<Decimal>,
    dose_unit: Option<String>,
    frequency: Option<String>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    notes: Option<String>,
    max_daily_doses: Option<i32>,
    min_hours_between_doses: Option<i32>,
    dose_cycle: Option<i32>,
    schedule_type: i32,
    schedule_config: Value,
}

impl ScheduleFields {
    fn new() -> Self {
        Self {
            person_id: None,
            medication_id: None,
            source_dosage_option_id: None,
            dose_amount: None,
            dose_unit: None,
            frequency: None,
            start_date: None,
            end_date: None,
            notes: None,
            max_daily_doses: Some(4),
            min_hours_between_doses: None,
            dose_cycle: None,
            schedule_type: 0,
            schedule_config: json!({}),
        }
    }

    fn validate(&self) -> Result<(), InputFailure> {
        let mut errors = Map::new();
        if self.person_id.is_none() {
            errors.insert("person_id".to_owned(), json!(["can't be blank"]));
        }
        if self.medication_id.is_none() {
            errors.insert("medication_id".to_owned(), json!(["can't be blank"]));
        }
        if self.dose_amount.is_none() {
            errors.insert("dose_amount".to_owned(), json!(["can't be blank"]));
        }
        if self.dose_unit.is_none() {
            errors.insert("dose_unit".to_owned(), json!(["can't be blank"]));
        }
        if self.start_date.is_none() {
            errors.insert("start_date".to_owned(), json!(["can't be blank"]));
        }
        if self.end_date.is_none() {
            errors.insert("end_date".to_owned(), json!(["can't be blank"]));
        }
        if let (Some(start), Some(end)) = (self.start_date, self.end_date)
            && end < start
        {
            errors.insert(
                "end_date".to_owned(),
                json!(["must be after the start date"]),
            );
        }
        if !errors.is_empty() {
            return Err(InputFailure::InvalidFields(Value::Object(errors)));
        }
        Ok(())
    }

    fn assign(self, active: &mut schedule::ActiveModel) {
        active.person_id = Set(self.person_id.unwrap());
        active.medication_id = Set(self.medication_id.unwrap());
        active.source_dosage_option_id = Set(self.source_dosage_option_id);
        active.dose_amount = Set(self.dose_amount);
        active.dose_unit = Set(self.dose_unit);
        active.frequency = Set(self.frequency);
        active.start_date = Set(self.start_date);
        active.end_date = Set(self.end_date);
        active.notes = Set(self.notes);
        active.max_daily_doses = Set(self.max_daily_doses);
        active.min_hours_between_doses = Set(self.min_hours_between_doses);
        active.dose_cycle = Set(self.dose_cycle);
        active.schedule_type = Set(self.schedule_type);
        active.schedule_config = Set(self.schedule_config);
    }
}
