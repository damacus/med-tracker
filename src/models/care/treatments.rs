use loco_rs::model::TenantQueryExt;
mod command;
mod input;
pub mod lifecycle;
pub(crate) mod projection;
mod retirement;
mod source_stock;
use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    care::{
        administration, medications,
        treatment_command::{DosageOptionId, DoseCycle, MedicationId, PersonId, ScheduleType},
        valid_identifier,
    },
    entities::{api_change_event, dosage, household, medication, person, schedule},
    errors::OperationError,
    identity::credentials::CredentialProvenance,
};
use chrono::{NaiveDate, Utc};
pub use retirement::retire;
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
    let query = crate::models::access::person_queries::household_people(context.scope());
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
    let query = dosage::Entity::find().in_tenant(context.scope().household_id);
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
    lock_household(tenant).await?;
    let attributes = input::attributes(body).map_err(failure)?;
    let id = attributes
        .get("person_id")
        .ok_or_else(|| failure(InputFailure::Invalid("person_id", "can't be blank")))?;
    let id = input::identifier(id, "person_id").map_err(failure)?;
    authorize_person_id_after_lock(tenant, id).await
}
async fn authorize_person_id(tenant: &TenantTransaction, id: &str) -> Result<(), OperationError> {
    lock_household(tenant).await?;
    authorize_person_id_after_lock(tenant, id).await
}
async fn lock_household(tenant: &TenantTransaction) -> Result<(), OperationError> {
    household::Entity::find_by_id(tenant.scope().household_id)
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    access::recheck(tenant).await?;
    Ok(())
}
async fn authorize_person_id_after_lock(
    tenant: &TenantTransaction,
    id: &str,
) -> Result<(), OperationError> {
    if !valid_identifier(id) {
        return Err(failure(InputFailure::Invalid("person_id", "is invalid")));
    }
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
    let input = input::parse_input(attributes)
        .map_err(failure)?
        .validated()?;
    let mut fields = ScheduleFields::new();
    command::apply_input(tenant.transaction(), tenant, &input, &mut fields, None)
        .await?
        .map_err(failure)?;
    fields.validate().map_err(failure)?;
    create_fields(tenant, fields, provenance).await
}
pub(crate) async fn create_input(
    tenant: &TenantTransaction,
    input: ValidatedScheduleInput,
    provenance: Option<&CredentialProvenance>,
) -> Result<(Value, String), OperationError> {
    let id = input
        .as_ref()
        .person_id
        .as_ref()
        .ok_or_else(|| failure(InputFailure::Invalid("person_id", "can't be blank")))?;
    authorize_person_id(tenant, id.as_str()).await?;
    let mut fields = ScheduleFields::new();
    command::apply_input(tenant.transaction(), tenant, &input, &mut fields, None)
        .await?
        .map_err(failure)?;
    fields.validate().map_err(failure)?;
    create_fields(tenant, fields, provenance).await
}
async fn create_fields(
    tenant: &TenantTransaction,
    fields: ScheduleFields,
    provenance: Option<&CredentialProvenance>,
) -> Result<(Value, String), OperationError> {
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
    let actor =
        crate::models::entities::account::Entity::find_by_id(tenant.scope().actor.account_id)
            .one(tenant.transaction())
            .await?
            .ok_or(OperationError::Unauthenticated)?;
    let zone = crate::models::identity::time_zone::preferred(&actor.preferences, tenant.settings())
        .map_err(|_| OperationError::Unavailable)?;
    let row = super::person_schedules::save_execution(
        tenant,
        row,
        Some(zone),
        super::doses::app_zone(tenant.settings()),
        provenance,
    )
    .await?;
    persist(tenant, &row, None, "create", provenance).await?;
    lifecycle::project(tenant, row).await
}
pub(super) async fn persist(
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
pub(crate) fn representation(row: &schedule::Model) -> (Value, String) {
    let body = json!({"data":{"id":row.id,"portable_id":row.portable_id,"person_id":row.person_id,"person_schedule_id":row.person_schedule_id,"clinical_revision":row.clinical_revision,"origin_time_zone":row.origin_time_zone,"execution_ended_at":row.execution_ended_at,"retired_at":row.retired_at,"medication_id":row.medication_id,"source_dosage_option_id":row.source_dosage_option_id,"active":row.active,"dose_amount":row.dose_amount.map(|value|medications::decimal_string(value.to_string())),"dose_unit":row.dose_unit,"frequency":row.frequency,"start_date":row.start_date,"end_date":row.end_date,"notes":row.notes,"max_daily_doses":row.max_daily_doses,"min_hours_between_doses":row.min_hours_between_doses.map(|value|format!("{value}.0")),"dose_cycle":row.dose_cycle.map(|value|match value{1=>"weekly",2=>"monthly",_=>"daily"}),"schedule_type":match row.schedule_type{1=>"multiple_daily",2=>"weekly",3=>"specific_dates",4=>"prn",5=>"tapering",6=>"every_other_day",_=>"daily"},"schedule_config":row.schedule_config,"updated_at":row.updated_at}});
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
pub(super) fn supported_unit(unit: &str) -> bool {
    UNITS.contains(&unit)
}
const FIELDS: &[&str] = &[
    "origin_time_zone",
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

#[derive(Default)]
pub(crate) struct ScheduleInput {
    pub(crate) origin_time_zone: Option<chrono_tz::Tz>,
    pub(crate) person_id: Option<PersonId>,
    pub(crate) medication_id: Option<MedicationId>,
    pub(crate) source_dosage_option_id: Option<DosageOptionId>,
    pub(crate) dose_amount: Option<Decimal>,
    pub(crate) dose_unit: Option<String>,
    pub(crate) frequency: Option<String>,
    pub(crate) start_date: Option<NaiveDate>,
    pub(crate) end_date: Option<NaiveDate>,
    pub(crate) notes: Option<String>,
    pub(crate) max_daily_doses: Option<i32>,
    pub(crate) min_hours_between_doses: Option<Option<i32>>,
    pub(crate) dose_cycle: Option<DoseCycle>,
    pub(crate) schedule_type: Option<ScheduleType>,
    pub(crate) schedule_config: Option<Value>,
}
pub(crate) struct ValidatedScheduleInput(ScheduleInput);

pub(crate) fn parse_personal_input(body: &Value) -> Result<ValidatedScheduleInput, OperationError> {
    let attributes = input::attributes(body).map_err(failure)?;
    input::parse_input(attributes).map_err(failure)?.validated()
}

pub(crate) fn amend_personal(
    row: &mut crate::models::entities::person_schedule::Model,
    input: &ValidatedScheduleInput,
) -> Result<(), OperationError> {
    let input = input.as_ref();
    if input.person_id.is_some()
        || input.medication_id.is_some()
        || input.source_dosage_option_id.is_some()
    {
        return Err(failure(InputFailure::Invalid(
            "schedule",
            "personal edits cannot select household stock",
        )));
    }
    if let Some(value) = input.dose_amount {
        row.dose_amount = Some(value);
    }
    if let Some(value) = input.dose_unit.as_ref() {
        row.dose_unit = Some(value.clone());
    }
    if let Some(value) = input.frequency.as_ref() {
        row.frequency = Some(value.clone());
    }
    if let Some(value) = input.notes.as_ref() {
        row.notes = Some(value.clone());
    }
    if let Some(value) = input.start_date {
        row.start_date = Some(value);
    }
    if let Some(value) = input.end_date {
        row.end_date = Some(value);
    }
    if let Some(value) = input.max_daily_doses {
        row.max_daily_doses = Some(value);
    }
    if let Some(value) = input.min_hours_between_doses {
        row.min_hours_between_doses = value;
    }
    if let Some(value) = input.dose_cycle {
        row.dose_cycle = Some(value.database());
    }
    if let Some(value) = input.schedule_type {
        row.schedule_type = value.database();
    }
    if let Some(value) = input.schedule_config.as_ref() {
        row.schedule_config = value.clone();
    }
    if let Some(value) = input.origin_time_zone
        && row.origin_time_zone != value.name()
    {
        row.origin_time_zone = value.name().into();
        row.timezone_provenance = "authorised_correction".into();
    }
    ScheduleFields {
        origin_time_zone: input.origin_time_zone,
        person_id: Some(row.person_id),
        medication_id: Some(row.origin_stock_id),
        source_dosage_option_id: None,
        dose_amount: row.dose_amount,
        dose_unit: row.dose_unit.clone(),
        frequency: row.frequency.clone(),
        start_date: row.start_date,
        end_date: row.end_date,
        notes: row.notes.clone(),
        max_daily_doses: row.max_daily_doses,
        min_hours_between_doses: row.min_hours_between_doses,
        dose_cycle: row.dose_cycle,
        schedule_type: row.schedule_type,
        schedule_config: row.schedule_config.clone(),
    }
    .validate()
    .map_err(failure)
}

pub(crate) fn amend_shared(
    row: &mut crate::models::entities::shared_schedule::Model,
    input: &ValidatedScheduleInput,
) -> Result<(), OperationError> {
    let input = input.as_ref();
    if input.person_id.is_some()
        || input.medication_id.is_some()
        || input.source_dosage_option_id.is_some()
        || input.notes.is_some()
    {
        return Err(failure(InputFailure::Invalid(
            "schedule",
            "shared clinical edits cannot select inventory or private notes",
        )));
    }
    if let Some(value) = input.dose_amount {
        row.dose_amount = Some(value);
    }
    if let Some(value) = input.dose_unit.as_ref() {
        row.dose_unit = Some(value.clone());
    }
    if let Some(value) = input.frequency.as_ref() {
        row.frequency = Some(value.clone());
    }
    if let Some(value) = input.start_date {
        row.start_date = Some(value);
    }
    if let Some(value) = input.end_date {
        row.end_date = Some(value);
    }
    if let Some(value) = input.max_daily_doses {
        row.max_daily_doses = Some(value);
    }
    if let Some(value) = input.min_hours_between_doses {
        row.min_hours_between_doses = value;
    }
    if let Some(value) = input.dose_cycle {
        row.dose_cycle = Some(value.database());
    }
    if let Some(value) = input.schedule_type {
        row.schedule_type = value.database();
    }
    if let Some(value) = input.schedule_config.as_ref() {
        row.schedule_config = value.clone();
    }
    if let Some(value) = input.origin_time_zone {
        row.origin_time_zone = value.name().into();
    }
    ScheduleFields {
        origin_time_zone: input.origin_time_zone,
        person_id: Some(row.person_id),
        medication_id: None,
        source_dosage_option_id: None,
        dose_amount: row.dose_amount,
        dose_unit: row.dose_unit.clone(),
        frequency: row.frequency.clone(),
        start_date: row.start_date,
        end_date: row.end_date,
        notes: None,
        max_daily_doses: row.max_daily_doses,
        min_hours_between_doses: row.min_hours_between_doses,
        dose_cycle: row.dose_cycle,
        schedule_type: row.schedule_type,
        schedule_config: row.schedule_config.clone(),
    }
    .validate_fields(false)
    .map_err(failure)
}

pub(crate) fn prescribed_amount(value: &str) -> Result<Decimal, OperationError> {
    let amount = Decimal::from_str(value).map_err(|_| {
        failure(InputFailure::Invalid(
            "dose_amount",
            "must be a decimal string",
        ))
    })?;
    if amount <= Decimal::ZERO || storage_decimal(amount, 8, 2).is_none() {
        return Err(failure(InputFailure::Invalid(
            "dose_amount",
            "must fit two decimal places",
        )));
    }
    Ok(amount)
}

impl ScheduleInput {
    pub(crate) fn validated(self) -> Result<ValidatedScheduleInput, OperationError> {
        if self.dose_amount.is_some_and(|amount| {
            amount <= Decimal::ZERO || storage_decimal(amount, 8, 2).is_none()
        }) {
            return Err(failure(InputFailure::Invalid(
                "dose_amount",
                "must fit two decimal places",
            )));
        }
        if self
            .dose_unit
            .as_ref()
            .is_some_and(|unit| !UNITS.contains(&unit.as_str()))
        {
            return Err(failure(InputFailure::Invalid("dose_unit", "is invalid")));
        }
        if self.max_daily_doses.is_some_and(|value| value <= 0) {
            return Err(failure(InputFailure::Invalid(
                "max_daily_doses",
                "must be positive",
            )));
        }
        if self
            .min_hours_between_doses
            .is_some_and(|value| value.is_some_and(|hours| hours <= 0))
        {
            return Err(failure(InputFailure::Invalid(
                "min_hours_between_doses",
                "must be a whole positive number",
            )));
        }
        if self
            .schedule_config
            .as_ref()
            .is_some_and(|config| !input::valid_config(config))
        {
            return Err(failure(InputFailure::Invalid(
                "schedule_config",
                "is invalid",
            )));
        }
        Ok(ValidatedScheduleInput(self))
    }
}

impl ValidatedScheduleInput {
    pub(crate) fn as_ref(&self) -> &ScheduleInput {
        &self.0
    }
}

#[derive(PartialEq)]
struct ScheduleFields {
    origin_time_zone: Option<chrono_tz::Tz>,
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
            origin_time_zone: None,
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
        self.validate_fields(true)
    }

    fn validate_fields(&self, local_stock: bool) -> Result<(), InputFailure> {
        let mut errors = Map::new();
        if self.person_id.is_none() {
            errors.insert("person_id".to_owned(), json!(["can't be blank"]));
        }
        if local_stock && self.medication_id.is_none() {
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
        if let Some(zone) = self.origin_time_zone {
            active.origin_time_zone = Set(Some(zone.name().into()));
        }
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
