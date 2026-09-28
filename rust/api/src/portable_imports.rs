use crate::entities::{grant, security_audit_event};
use crate::medication_management::{
    error_response, finish_with_request_id, household_manager, record_version, request_context,
};
use crate::mutation_idempotency::lock_household_and_reauthenticate;
use crate::portable_crypto::{self, PortableCryptoError};
use crate::sync_events::{record_change, SyncRecord};
use crate::{database_error, ApiError, AppState, AuthContext};
use axum::extract::{rejection::JsonRejection, DefaultBodyLimit, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::post;
use axum::{Json, Router};
use chrono::Utc;
use chrono::{DateTime, Datelike, NaiveDate, NaiveTime};
use sea_orm::prelude::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, DbBackend, EntityTrait,
    QueryFilter, Set, Statement, TransactionTrait,
};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use uuid::Uuid;

const CONTROLLER: &str = "api/v1/portable_imports";
const PASSPHRASE_HEADER: &str = "x-medtracker-portable-passphrase";
const BODY_LIMIT: usize = 24 * 1024 * 1024;
const BASE_TYPES: &[&str] = &[
    "people",
    "locations",
    "medications",
    "dosage_options",
    "schedules",
    "person_medications",
    "medication_takes",
    "notification_preferences",
];
const V2_TYPES: &[&str] = &[
    "medication_pause_periods",
    "dose_occurrences",
    "health_events",
];

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/households/{household_id}/portable_imports/dry_run",
            post(dry_run),
        )
        .route(
            "/api/v1/households/{household_id}/portable_imports",
            post(apply),
        )
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
}

async fn dry_run(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    body: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    handle(state, household_id, headers, body, true).await
}

async fn apply(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    body: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    handle(state, household_id, headers, body, false).await
}

async fn fail(
    db: DatabaseTransaction,
    context: &AuthContext,
    action: &str,
    status: StatusCode,
    message: &str,
) -> Result<Response, ApiError> {
    let code = match status {
        StatusCode::BAD_REQUEST => "bad_request",
        StatusCode::FORBIDDEN => "forbidden",
        _ => "unprocessable_content",
    };
    error_response(
        db,
        context,
        "POST",
        CONTROLLER,
        "HouseholdPolicy",
        action,
        status,
        code,
        message,
        None,
    )
    .await
}

fn passphrase(headers: &HeaderMap) -> Option<String> {
    headers
        .get(PASSPHRASE_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
}

fn crypto_message(error: PortableCryptoError) -> &'static str {
    match error {
        PortableCryptoError::TooLarge => "Portable bundle is too large",
        PortableCryptoError::InvalidEnvelope => "Portable bundle could not be decrypted",
        PortableCryptoError::Unavailable => "Portable decryption is temporarily unavailable",
    }
}

async fn handle(
    state: AppState,
    household_id: i64,
    headers: HeaderMap,
    body: Result<Json<Value>, JsonRejection>,
    is_dry_run: bool,
) -> Result<Response, ApiError> {
    let action = if is_dry_run { "dry_run" } else { "create" };
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let Json(body) = match body {
        Ok(body) => body,
        Err(rejection) => {
            let status = if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
                StatusCode::UNPROCESSABLE_ENTITY
            } else {
                StatusCode::BAD_REQUEST
            };
            let message = if status == StatusCode::UNPROCESSABLE_ENTITY {
                "Portable bundle is too large"
            } else {
                "Invalid JSON request body"
            };
            return fail(db, &context, action, status, message).await;
        }
    };
    let Some(object) = body.as_object() else {
        return fail(
            db,
            &context,
            action,
            StatusCode::BAD_REQUEST,
            "Portable bundle is required",
        )
        .await;
    };
    let Some(bundle) = object.get("bundle") else {
        return fail(
            db,
            &context,
            action,
            StatusCode::BAD_REQUEST,
            "Portable bundle is required",
        )
        .await;
    };
    if object.len() != 1 {
        return fail(
            db,
            &context,
            action,
            StatusCode::UNPROCESSABLE_ENTITY,
            "Unsupported import request fields",
        )
        .await;
    }
    let Some(passphrase) = passphrase(&headers) else {
        return fail(
            db,
            &context,
            action,
            StatusCode::UNPROCESSABLE_ENTITY,
            "Portable passphrase header is required",
        )
        .await;
    };
    db.commit().await.map_err(database_error)?;
    let bundle = bundle.clone();
    let decrypted =
        tokio::task::spawn_blocking(move || portable_crypto::decrypt(&bundle, &passphrase))
            .await
            .map_err(|_| ApiError::internal())?;
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let payload = match decrypted {
        Ok(payload) => payload,
        Err(error) => {
            return fail(
                db,
                &context,
                action,
                StatusCode::UNPROCESSABLE_ENTITY,
                crypto_message(error),
            )
            .await
        }
    };
    let plan = match ImportPlan::parse(payload) {
        Ok(plan) => plan,
        Err(message) => {
            return fail(
                db,
                &context,
                action,
                StatusCode::UNPROCESSABLE_ENTITY,
                &message,
            )
            .await
        }
    };
    db.rollback().await.map_err(database_error)?;
    let db = state.db.begin().await.map_err(database_error)?;
    let (_, context) =
        lock_household_and_reauthenticate(&state, &db, &headers, household_id).await?;
    let mut index = ExistingIndex::load(&db, household_id).await?;
    if !authorized(&db, &context, &plan, &index).await? {
        return fail(
            db,
            &context,
            action,
            StatusCode::FORBIDDEN,
            "You are not authorized to perform this action.",
        )
        .await;
    }
    let result = preflight(&db, &context, &plan, &index).await?;
    if is_dry_run || !result.conflicts.is_empty() || !result.errors.is_empty() {
        let status = if is_dry_run {
            StatusCode::OK
        } else {
            StatusCode::UNPROCESSABLE_ENTITY
        };
        return finish_with_request_id(
            db,
            &context,
            &Uuid::new_v4().to_string(),
            "POST",
            CONTROLLER,
            "HouseholdPolicy",
            action,
            status,
            status.is_success(),
            json!({"data": result.body(false)}),
            None,
        )
        .await;
    }
    let request_id = Uuid::new_v4().to_string();
    if let Err(message) = write_records(&db, &context, &plan, &mut index, &request_id).await {
        db.rollback().await.map_err(database_error)?;
        let (fresh, context) = request_context(&state, &headers, household_id).await?;
        return fail(
            fresh,
            &context,
            action,
            StatusCode::UNPROCESSABLE_ENTITY,
            &message,
        )
        .await;
    }
    let now = Utc::now().naive_utc();
    security_audit_event::ActiveModel {
        household_id: Set(household_id),
        actor_account_id: Set(Some(context.account_id)),
        actor_membership_id: Set(Some(context.membership.id)),
        event_type: Set("portable_data.imported".to_owned()),
        request_id: Set(Some(request_id.clone())),
        ip: Set(None),
        metadata: Set(json!({"record_counts": plan.counts, "dry_run": false})),
        audit_context: Set(
            json!({"request_id": request_id, "actor_membership_id": context.membership.id}),
        ),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .map_err(database_error)?;
    finish_with_request_id(
        db,
        &context,
        &request_id,
        "POST",
        CONTROLLER,
        "HouseholdPolicy",
        action,
        StatusCode::CREATED,
        true,
        json!({"data": result.body(true)}),
        None,
    )
    .await
}

struct ImportPlan {
    records: Map<String, Value>,
    counts: Value,
    errors: Vec<String>,
    v2: bool,
}

impl ImportPlan {
    fn parse(payload: Value) -> Result<Self, String> {
        let object = payload
            .as_object()
            .ok_or("Portable payload must be an object")?;
        if object.keys().any(|key| {
            ![
                "format",
                "scope",
                "exported_at",
                "source_instance_id",
                "records",
            ]
            .contains(&key.as_str())
        }) {
            return Err("Unsupported portable payload fields".to_owned());
        }
        let format = payload["format"]
            .as_str()
            .ok_or("Portable data format is required")?;
        if !matches!(format, "medtracker.portable.v1" | "medtracker.portable.v2") {
            return Err("Unsupported portable data format".to_owned());
        }
        let records = payload["records"]
            .as_object()
            .ok_or("Portable data records are required")?;
        let mut counts = Map::new();
        let mut errors = Vec::new();
        for (kind, rows) in records {
            if !BASE_TYPES.contains(&kind.as_str())
                && !(format == "medtracker.portable.v2" && V2_TYPES.contains(&kind.as_str()))
            {
                return Err(format!("Unsupported portable record type: {kind}"));
            }
            let rows = rows
                .as_array()
                .ok_or_else(|| format!("{kind} must be an array"))?;
            let mut seen = HashSet::new();
            for (index, row) in rows.iter().enumerate() {
                let object = row
                    .as_object()
                    .ok_or_else(|| format!("{kind}[{index}] must be an object"))?;
                let portable_id = row["portable_id"]
                    .as_str()
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| format!("{kind}[{index}].portable_id is required"))?;
                if !seen.insert(portable_id) {
                    return Err(format!("{kind}[{index}].portable_id is duplicated"));
                }
                let allowed = allowed_fields(kind);
                let forbidden: Vec<&str> = object
                    .keys()
                    .filter(|key| {
                        key.as_str() == "id" || (key.ends_with("_id") && !key.contains("portable"))
                    })
                    .map(String::as_str)
                    .collect();
                if !forbidden.is_empty() {
                    errors.push(format!(
                        "{kind}[{index}] includes Rails numeric IDs: {}",
                        forbidden.join(", ")
                    ));
                }
                if object.keys().any(|key| {
                    !allowed.contains(&key.as_str()) && !forbidden.contains(&key.as_str())
                }) {
                    return Err(format!("{kind}[{index}] has unsupported fields"));
                }
                for field in required_fields(kind) {
                    if row[*field].is_null() || row[*field].as_str().is_some_and(str::is_empty) {
                        return Err(format!("{kind}[{index}].{field} is required"));
                    }
                }
            }
            counts.insert(kind.clone(), json!(rows.len()));
        }
        Ok(Self {
            records: records.clone(),
            counts: Value::Object(counts),
            errors,
            v2: format == "medtracker.portable.v2",
        })
    }

    fn rows(&self, kind: &str) -> &[Value] {
        self.records
            .get(kind)
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

fn allowed_fields(kind: &str) -> &'static [&'static str] {
    match kind {
        "people" => &[
            "portable_id",
            "updated_at",
            "etag",
            "name",
            "email",
            "date_of_birth",
            "person_type",
            "has_capacity",
            "location_portable_ids",
            "notification_preference_portable_id",
        ],
        "locations" => &["portable_id", "updated_at", "etag", "name", "description"],
        "medications" => &[
            "portable_id",
            "updated_at",
            "etag",
            "location_portable_id",
            "name",
            "friendly_name",
            "category",
            "description",
            "dose_amount",
            "dose_unit",
            "default_schedule_type",
            "current_supply",
            "reorder_threshold",
            "barcode",
            "dmd_code",
            "dmd_system",
            "dmd_concept_class",
        ],
        "dosage_options" => &[
            "portable_id",
            "updated_at",
            "etag",
            "medication_portable_id",
            "amount",
            "unit",
            "frequency",
            "description",
            "default_for_adults",
            "default_for_children",
            "default_max_daily_doses",
            "default_min_hours_between_doses",
            "default_dose_cycle",
            "current_supply",
            "reorder_threshold",
        ],
        "schedules" => &[
            "portable_id",
            "updated_at",
            "etag",
            "source_dosage_option_portable_id",
            "retired_at",
            "person_portable_id",
            "medication_portable_id",
            "dose_amount",
            "dose_unit",
            "frequency",
            "dose_cycle",
            "max_daily_doses",
            "min_hours_between_doses",
            "schedule_type",
            "schedule_config",
            "start_date",
            "end_date",
            "active",
            "notes",
        ],
        "person_medications" => &[
            "portable_id",
            "updated_at",
            "etag",
            "source_dosage_option_portable_id",
            "retired_at",
            "person_portable_id",
            "medication_portable_id",
            "dose_amount",
            "dose_unit",
            "dose_cycle",
            "max_daily_doses",
            "min_hours_between_doses",
            "administration_kind",
            "active",
            "notes",
            "position",
        ],
        "medication_takes" => &[
            "portable_id",
            "updated_at",
            "etag",
            "client_uuid",
            "source_type",
            "source_portable_id",
            "taken_at",
            "dose_amount",
            "dose_unit",
            "taken_from_medication_portable_id",
            "taken_from_location_portable_id",
        ],
        "notification_preferences" => &[
            "portable_id",
            "updated_at",
            "etag",
            "person_portable_id",
            "enabled",
            "dose_due_enabled",
            "missed_dose_enabled",
            "low_stock_enabled",
            "private_text_enabled",
            "morning_time",
            "afternoon_time",
            "evening_time",
            "night_time",
        ],
        "medication_pause_periods" => &[
            "portable_id",
            "updated_at",
            "etag",
            "source_type",
            "source_portable_id",
            "reason",
            "note",
            "started_at",
            "ended_at",
            "created_at",
            "legacy_context",
            "imported_context",
            "recorded_by_person_portable_id",
            "resumed_by_person_portable_id",
        ],
        "dose_occurrences" => &[
            "portable_id",
            "updated_at",
            "etag",
            "source_type",
            "source_portable_id",
            "window_starts_on",
            "window_ends_on",
            "position",
            "scheduled_at",
            "outcome",
            "reason",
            "note",
            "resolved_at",
            "medication_take_portable_id",
        ],
        "health_events" => &[
            "portable_id",
            "updated_at",
            "etag",
            "person_portable_id",
            "event_kind",
            "severity",
            "title",
            "notes",
            "started_on",
            "ended_on",
            "medication_portable_ids",
        ],
        _ => &[],
    }
}

fn required_fields(kind: &str) -> &'static [&'static str] {
    match kind {
        "people" => &["name", "date_of_birth", "person_type", "has_capacity"],
        "locations" => &["name"],
        "medications" => &[
            "name",
            "location_portable_id",
            "default_schedule_type",
            "reorder_threshold",
        ],
        "dosage_options" => &[
            "medication_portable_id",
            "amount",
            "unit",
            "frequency",
            "default_max_daily_doses",
            "default_min_hours_between_doses",
            "default_dose_cycle",
        ],
        "schedules" => &[
            "person_portable_id",
            "medication_portable_id",
            "schedule_type",
            "schedule_config",
            "start_date",
            "end_date",
            "dose_amount",
            "dose_unit",
            "active",
        ],
        "person_medications" => &[
            "person_portable_id",
            "medication_portable_id",
            "administration_kind",
            "dose_amount",
            "dose_unit",
            "active",
            "position",
        ],
        "medication_takes" => &[
            "source_type",
            "source_portable_id",
            "taken_at",
            "dose_amount",
            "dose_unit",
        ],
        "notification_preferences" => &["person_portable_id"],
        "medication_pause_periods" => &["source_type", "source_portable_id", "reason"],
        "dose_occurrences" => &[
            "source_type",
            "source_portable_id",
            "window_starts_on",
            "position",
            "outcome",
        ],
        "health_events" => &["person_portable_id", "event_kind", "title", "started_on"],
        _ => &[],
    }
}

struct Preflight {
    counts: Value,
    conflicts: Vec<Value>,
    errors: Vec<String>,
}

impl Preflight {
    fn body(&self, applied: bool) -> Value {
        json!({"applied": applied, "counts": self.counts, "conflicts": self.conflicts, "errors": self.errors})
    }
}

struct ExistingIndex {
    ids: HashMap<&'static str, HashMap<String, i64>>,
    names: HashMap<&'static str, HashMap<String, String>>,
    owners: HashMap<&'static str, HashMap<String, String>>,
    source_medications: HashMap<&'static str, HashMap<String, String>>,
    dosage_medications: HashMap<String, String>,
    dosage_values: HashMap<String, (String, String)>,
    medication_locations: HashMap<String, String>,
    take_sources: HashMap<String, (String, String)>,
}

impl ExistingIndex {
    async fn load(db: &DatabaseTransaction, household_id: i64) -> Result<Self, ApiError> {
        let mut ids = HashMap::new();
        let mut names = HashMap::new();
        for kind in BASE_TYPES.iter().chain(V2_TYPES) {
            let table = table(kind);
            let sql = if matches!(*kind, "locations" | "medications" | "people") {
                format!(
                    "SELECT id, portable_id, {} AS match_name FROM {table} WHERE household_id = $1",
                    if *kind == "people" {
                        "lower(email)"
                    } else {
                        "lower(name)"
                    }
                )
            } else {
                format!("SELECT id, portable_id FROM {table} WHERE household_id = $1")
            };
            let rows = db
                .query_all_raw(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    sql,
                    [household_id.into()],
                ))
                .await
                .map_err(database_error)?;
            let mut type_ids = HashMap::new();
            let mut type_names = HashMap::new();
            for row in rows {
                let portable_id: String = row.try_get("", "portable_id").map_err(database_error)?;
                let id: i64 = row.try_get("", "id").map_err(database_error)?;
                if matches!(*kind, "locations" | "medications" | "people") {
                    if let Ok(Some(name)) = row.try_get::<Option<String>>("", "match_name") {
                        type_names.insert(name, portable_id.clone());
                    }
                }
                type_ids.insert(portable_id, id);
            }
            ids.insert(*kind, type_ids);
            names.insert(*kind, type_names);
        }
        let mut owners = HashMap::new();
        for kind in [
            "schedules",
            "person_medications",
            "notification_preferences",
            "health_events",
            "medication_takes",
            "medication_pause_periods",
            "dose_occurrences",
        ] {
            let table = table(kind);
            let sql = if matches!(
                kind,
                "schedules" | "person_medications" | "notification_preferences" | "health_events"
            ) {
                format!("SELECT record.portable_id, person.portable_id AS person_portable_id FROM {table} record JOIN people person ON person.id = record.person_id AND person.household_id = record.household_id WHERE record.household_id = $1")
            } else {
                format!("SELECT record.portable_id, person.portable_id AS person_portable_id FROM {table} record LEFT JOIN schedules schedule ON schedule.id = record.schedule_id AND schedule.household_id = record.household_id LEFT JOIN person_medications assignment ON assignment.id = record.person_medication_id AND assignment.household_id = record.household_id JOIN people person ON person.id = COALESCE(schedule.person_id, assignment.person_id) AND person.household_id = record.household_id WHERE record.household_id = $1")
            };
            let rows = db
                .query_all_raw(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    sql,
                    [household_id.into()],
                ))
                .await
                .map_err(database_error)?;
            let mut type_owners = HashMap::new();
            for row in rows {
                let portable_id: String = row.try_get("", "portable_id").map_err(database_error)?;
                let owner: String = row
                    .try_get("", "person_portable_id")
                    .map_err(database_error)?;
                type_owners.insert(portable_id, owner);
            }
            owners.insert(kind, type_owners);
        }
        let mut source_medications = HashMap::new();
        for kind in ["schedules", "person_medications"] {
            let sql = format!("SELECT source.portable_id, medication.portable_id AS medication_portable_id FROM {} source JOIN medications medication ON medication.id = source.medication_id AND medication.household_id = source.household_id WHERE source.household_id = $1", table(kind));
            let rows = db
                .query_all_raw(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    sql,
                    [household_id.into()],
                ))
                .await
                .map_err(database_error)?;
            let mut mapped = HashMap::new();
            for row in rows {
                mapped.insert(
                    row.try_get("", "portable_id").map_err(database_error)?,
                    row.try_get("", "medication_portable_id")
                        .map_err(database_error)?,
                );
            }
            source_medications.insert(kind, mapped);
        }
        let rows = db.query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT dosage.portable_id, medication.portable_id AS medication_portable_id, dosage.amount::text AS amount, dosage.unit FROM dosages dosage JOIN medications medication ON medication.id = dosage.medication_id AND medication.household_id = dosage.household_id WHERE dosage.household_id = $1", [household_id.into()]))
            .await.map_err(database_error)?;
        let mut dosage_medications = HashMap::new();
        let mut dosage_values = HashMap::new();
        for row in rows {
            let portable_id: String = row.try_get("", "portable_id").map_err(database_error)?;
            dosage_medications.insert(
                portable_id.clone(),
                row.try_get("", "medication_portable_id")
                    .map_err(database_error)?,
            );
            dosage_values.insert(
                portable_id,
                (
                    row.try_get("", "amount").map_err(database_error)?,
                    row.try_get("", "unit").map_err(database_error)?,
                ),
            );
        }
        let rows = db.query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT medication.portable_id, location.portable_id AS location_portable_id FROM medications medication JOIN locations location ON location.id = medication.location_id AND location.household_id = medication.household_id WHERE medication.household_id = $1", [household_id.into()]))
            .await.map_err(database_error)?;
        let mut medication_locations = HashMap::new();
        for row in rows {
            medication_locations.insert(
                row.try_get("", "portable_id").map_err(database_error)?,
                row.try_get("", "location_portable_id")
                    .map_err(database_error)?,
            );
        }
        let rows = db.query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT take.portable_id, CASE WHEN take.schedule_id IS NOT NULL THEN 'schedule' ELSE 'person_medication' END AS source_type, COALESCE(schedule.portable_id, assignment.portable_id) AS source_portable_id FROM medication_takes take LEFT JOIN schedules schedule ON schedule.id = take.schedule_id AND schedule.household_id = take.household_id LEFT JOIN person_medications assignment ON assignment.id = take.person_medication_id AND assignment.household_id = take.household_id WHERE take.household_id = $1", [household_id.into()]))
            .await.map_err(database_error)?;
        let mut take_sources = HashMap::new();
        for row in rows {
            take_sources.insert(
                row.try_get("", "portable_id").map_err(database_error)?,
                (
                    row.try_get("", "source_type").map_err(database_error)?,
                    row.try_get("", "source_portable_id")
                        .map_err(database_error)?,
                ),
            );
        }
        Ok(Self {
            ids,
            names,
            owners,
            source_medications,
            dosage_medications,
            dosage_values,
            medication_locations,
            take_sources,
        })
    }

    fn has(&self, plan: &ImportPlan, kind: &str, portable_id: &str) -> bool {
        self.ids
            .get(kind)
            .is_some_and(|ids| ids.contains_key(portable_id))
            || plan
                .rows(kind)
                .iter()
                .any(|row| row["portable_id"] == portable_id)
    }

    fn get(&self, kind: &str, portable_id: &str) -> Option<i64> {
        self.ids
            .get(kind)
            .and_then(|ids| ids.get(portable_id))
            .copied()
    }

    fn put(&mut self, kind: &'static str, portable_id: String, id: i64) {
        self.ids.entry(kind).or_default().insert(portable_id, id);
    }

    fn owner(&self, kind: &str, portable_id: &str) -> Option<&str> {
        self.owners
            .get(kind)
            .and_then(|owners| owners.get(portable_id))
            .map(String::as_str)
    }

    fn source_medication<'a>(
        &'a self,
        plan: &'a ImportPlan,
        kind: &str,
        portable_id: &str,
    ) -> Option<&'a str> {
        plan.rows(kind)
            .iter()
            .find(|row| row["portable_id"] == portable_id)
            .and_then(|row| row["medication_portable_id"].as_str())
            .or_else(|| {
                self.source_medications
                    .get(kind)
                    .and_then(|rows| rows.get(portable_id).map(String::as_str))
            })
    }

    fn dosage_medication<'a>(&'a self, plan: &'a ImportPlan, portable_id: &str) -> Option<&'a str> {
        plan.rows("dosage_options")
            .iter()
            .find(|row| row["portable_id"] == portable_id)
            .and_then(|row| row["medication_portable_id"].as_str())
            .or_else(|| self.dosage_medications.get(portable_id).map(String::as_str))
    }

    fn dosage_value<'a>(
        &'a self,
        plan: &'a ImportPlan,
        portable_id: &str,
    ) -> Option<(Decimal, &'a str)> {
        if let Some(row) = plan
            .rows("dosage_options")
            .iter()
            .find(|row| row["portable_id"] == portable_id)
        {
            return Some((decimal(&row["amount"])?, row["unit"].as_str()?));
        }
        let (amount, unit) = self.dosage_values.get(portable_id)?;
        Some((Decimal::from_str(amount).ok()?, unit))
    }

    fn medication_location<'a>(
        &'a self,
        plan: &'a ImportPlan,
        portable_id: &str,
    ) -> Option<&'a str> {
        plan.rows("medications")
            .iter()
            .find(|row| row["portable_id"] == portable_id)
            .and_then(|row| row["location_portable_id"].as_str())
            .or_else(|| {
                self.medication_locations
                    .get(portable_id)
                    .map(String::as_str)
            })
    }

    fn take_source<'a>(
        &'a self,
        plan: &'a ImportPlan,
        portable_id: &str,
    ) -> Option<(&'a str, &'a str)> {
        if let Some(row) = plan
            .rows("medication_takes")
            .iter()
            .find(|row| row["portable_id"] == portable_id)
        {
            return Some((
                row["source_type"].as_str()?,
                row["source_portable_id"].as_str()?,
            ));
        }
        self.take_sources
            .get(portable_id)
            .map(|(kind, id)| (kind.as_str(), id.as_str()))
    }
}

fn table(kind: &str) -> &'static str {
    match kind {
        "people" => "people",
        "locations" => "locations",
        "medications" => "medications",
        "dosage_options" => "dosages",
        "schedules" => "schedules",
        "person_medications" => "person_medications",
        "medication_takes" => "medication_takes",
        "notification_preferences" => "notification_preferences",
        "medication_pause_periods" => "medication_pause_periods",
        "dose_occurrences" => "medication_dose_occurrences",
        "health_events" => "health_events",
        _ => unreachable!(),
    }
}

fn decimal(value: &Value) -> Option<Decimal> {
    match value {
        Value::Number(number) => Decimal::from_str(&number.to_string()).ok(),
        Value::String(text) => Decimal::from_str(text).ok(),
        _ => None,
    }
}

fn validate_row(kind: &str, index: usize, row: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let object = row.as_object().expect("validated object");
    let label = format!("{kind}[{index}]");
    for field in [
        "name",
        "title",
        "unit",
        "frequency",
        "dose_unit",
        "source_type",
        "person_type",
        "administration_kind",
        "event_kind",
        "reason",
        "outcome",
    ] {
        if let Some(value) = object.get(field) {
            if !value.is_null() && value.as_str().is_none_or(|text| text.trim().is_empty()) {
                errors.push(format!("{label}.{field} must be nonblank text"));
            }
        }
    }
    for field in [
        "enabled",
        "dose_due_enabled",
        "missed_dose_enabled",
        "low_stock_enabled",
        "private_text_enabled",
        "has_capacity",
        "active",
        "default_for_adults",
        "default_for_children",
        "legacy_context",
    ] {
        if let Some(value) = object.get(field) {
            if !value.is_boolean() {
                errors.push(format!("{label}.{field} must be a boolean"));
            }
        }
    }
    for field in [
        "amount",
        "dose_amount",
        "current_supply",
        "reorder_threshold",
        "default_min_hours_between_doses",
    ] {
        if let Some(value) = object.get(field).filter(|value| !value.is_null()) {
            let valid = decimal(value).is_some_and(|number| {
                if matches!(field, "amount" | "dose_amount") {
                    number > Decimal::ZERO
                } else {
                    number >= Decimal::ZERO
                }
            });
            if !valid {
                errors.push(format!(
                    "{label}.{field} must be a valid nonnegative amount"
                ));
            }
        }
    }
    for field in [
        "default_max_daily_doses",
        "max_daily_doses",
        "min_hours_between_doses",
        "position",
    ] {
        if let Some(value) = object.get(field).filter(|value| !value.is_null()) {
            let valid = value.as_i64().is_some_and(|number| {
                if field == "min_hours_between_doses" {
                    number >= 0
                } else {
                    number > 0
                }
            });
            if !valid {
                errors.push(format!("{label}.{field} must be a valid integer"));
            }
        }
    }
    for field in [
        "date_of_birth",
        "start_date",
        "end_date",
        "started_on",
        "ended_on",
        "window_starts_on",
        "window_ends_on",
    ] {
        if let Some(value) = object.get(field).filter(|value| !value.is_null()) {
            if value
                .as_str()
                .and_then(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
                .is_none()
            {
                errors.push(format!("{label}.{field} must be an ISO date"));
            }
        }
    }
    for field in [
        "retired_at",
        "taken_at",
        "started_at",
        "ended_at",
        "scheduled_at",
        "resolved_at",
        "created_at",
    ] {
        if let Some(value) = object.get(field).filter(|value| !value.is_null()) {
            if value
                .as_str()
                .and_then(|time| DateTime::parse_from_rfc3339(time).ok())
                .is_none()
            {
                errors.push(format!("{label}.{field} must be an ISO timestamp"));
            }
        }
    }
    for field in [
        "morning_time",
        "afternoon_time",
        "evening_time",
        "night_time",
    ] {
        if let Some(value) = object.get(field).filter(|value| !value.is_null()) {
            if value
                .as_str()
                .and_then(|time| NaiveTime::parse_from_str(time, "%H:%M:%S").ok())
                .is_none()
            {
                errors.push(format!("{label}.{field} must be a time"));
            }
        }
    }
    for (field, choices) in [
        ("person_type", &["adult", "minor", "dependent_adult"][..]),
        (
            "default_schedule_type",
            &[
                "daily",
                "multiple_daily",
                "weekly",
                "specific_dates",
                "prn",
                "tapering",
                "every_other_day",
            ][..],
        ),
        (
            "schedule_type",
            &[
                "daily",
                "multiple_daily",
                "weekly",
                "specific_dates",
                "prn",
                "tapering",
                "every_other_day",
            ][..],
        ),
        ("dose_cycle", &["daily", "weekly", "monthly"][..]),
        ("default_dose_cycle", &["daily", "weekly", "monthly"][..]),
        ("administration_kind", &["routine", "as_needed"][..]),
        ("event_kind", &["illness", "suspected_side_effect"][..]),
        ("severity", &["mild", "moderate", "severe"][..]),
        ("source_type", &["schedule", "person_medication"][..]),
    ] {
        if let Some(value) = object.get(field).filter(|value| !value.is_null()) {
            if value.as_str().is_none_or(|value| !choices.contains(&value)) {
                errors.push(format!("{label}.{field} is unsupported"));
            }
        }
    }
    if let Some(name) = row["name"].as_str() {
        if name.trim().is_empty() {
            errors.push(format!("{label}.name is required"));
        }
    }
    if let Some(email) = row["email"].as_str().filter(|email| !email.is_empty()) {
        if email.contains(char::is_whitespace)
            || email.split('@').count() != 2
            || !email
                .split('@')
                .nth(1)
                .is_some_and(|domain| domain.contains('.'))
        {
            errors.push(format!("{label}.email is invalid"));
        }
    }
    if let Some(barcode) = row["barcode"].as_str().filter(|value| !value.is_empty()) {
        if !matches!(barcode.len(), 13 | 14) || !barcode.bytes().all(|byte| byte.is_ascii_digit()) {
            errors.push(format!("{label}.barcode is invalid"));
        }
    }
    if let Some(category) = row["category"].as_str().filter(|value| !value.is_empty()) {
        const CATEGORIES: &[&str] = &[
            "Analgesic",
            "Antibiotic",
            "Anticoagulant",
            "Anticonvulsant",
            "Antidepressant",
            "Antidiabetic",
            "Antiemetic",
            "Antifungal",
            "Antihistamine",
            "Antihypertensive",
            "Anti-Inflammatory",
            "Antiparasitic",
            "Antipsychotic",
            "Antiviral",
            "Anxiolytic",
            "Cardiovascular",
            "Cholesterol",
            "Contraceptive",
            "Dermatological",
            "Gastrointestinal",
            "Hormonal",
            "Immunosuppressant",
            "Migraine",
            "Mineral",
            "Muscle Relaxant",
            "Neurological",
            "Oncology",
            "Ophthalmic",
            "Osmotic Laxative",
            "Opioid",
            "Osteoporosis",
            "Respiratory",
            "Sleep Aid",
            "Smoking Cessation",
            "Supplement",
            "Thyroid",
            "Urological",
            "Vitamin",
            "Weight Management",
        ];
        if !CATEGORIES.contains(&category) {
            errors.push(format!("{label}.category is unsupported"));
        }
    }
    if let Some(unit) = row["dose_unit"].as_str().filter(|value| !value.is_empty()) {
        if ![
            "tablet", "capsule", "gummy", "mg", "ml", "g", "mcg", "IU", "spray", "drop", "sachet",
            "pad",
        ]
        .contains(&unit)
        {
            errors.push(format!("{label}.dose_unit is unsupported"));
        }
    }
    if row["dmd_code"]
        .as_str()
        .is_some_and(|value| !value.is_empty())
        && row["dmd_system"].as_str().is_none_or(str::is_empty)
    {
        errors.push(format!("{label}.dmd_system is required with dmd_code"));
    }
    if kind == "people" {
        if let Some(date) = row["date_of_birth"]
            .as_str()
            .and_then(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
        {
            let today = Utc::now().date_naive();
            let age = today.year()
                - date.year()
                - i32::from((today.month(), today.day()) < (date.month(), date.day()));
            if (age < 18 && row["person_type"] == "dependent_adult")
                || (age >= 18 && row["person_type"] == "minor")
            {
                errors.push(format!("{label}.person_type does not match age"));
            }
        }
    }
    if matches!(kind, "schedules" | "person_medications") {
        if let (Some(start), Some(end)) = (
            row["start_date"]
                .as_str()
                .and_then(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()),
            row["end_date"]
                .as_str()
                .and_then(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()),
        ) {
            if end < start {
                errors.push(format!("{label}.end_date must follow start_date"));
            }
        }
    }
    if kind == "dose_occurrences" {
        let outcome = row["outcome"].as_str().unwrap_or_default();
        if !["open", "taken", "not_taken"].contains(&outcome) {
            errors.push(format!("{label}.outcome is unsupported"));
        }
        if outcome == "open"
            && [
                "reason",
                "note",
                "resolved_at",
                "medication_take_portable_id",
            ]
            .iter()
            .any(|field| !row[*field].is_null())
        {
            errors.push(format!("{label} has invalid open outcome fields"));
        }
        if outcome == "taken" && row["medication_take_portable_id"].is_null() {
            errors.push(format!("{label}.medication_take_portable_id is required"));
        }
        if outcome != "open" && row["resolved_at"].is_null() {
            errors.push(format!("{label}.resolved_at is required"));
        }
    }
    if kind == "medication_pause_periods" {
        let reason = row["reason"].as_str().unwrap_or_default();
        if ![
            "out_of_supply",
            "temporarily_not_needed",
            "clinician_advice",
            "side_effects",
            "other",
            "reason_not_recorded",
        ]
        .contains(&reason)
        {
            errors.push(format!("{label}.reason is unsupported"));
        }
        let legacy = row["legacy_context"].as_bool().unwrap_or(false);
        if legacy != (reason == "reason_not_recorded") {
            errors.push(format!("{label}.legacy_context does not match reason"));
        }
        if !legacy && row["started_at"].is_null() {
            errors.push(format!("{label}.started_at is required"));
        }
    }
    if kind == "dose_occurrences" {
        if let Some(reason) = row["reason"].as_str() {
            if ![
                "refused",
                "unwell",
                "asleep",
                "medicine_unavailable",
                "clinician_advice",
                "other",
            ]
            .contains(&reason)
            {
                errors.push(format!("{label}.reason is unsupported"));
            }
        }
        if row["note"]
            .as_str()
            .is_some_and(|note| note.chars().count() > 2000)
        {
            errors.push(format!("{label}.note is too long"));
        }
        if row["outcome"] == "taken" && (!row["reason"].is_null() || !row["note"].is_null()) {
            errors.push(format!("{label} has invalid taken outcome fields"));
        }
        if let (Some(start), Some(end)) = (
            row["window_starts_on"]
                .as_str()
                .and_then(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()),
            row["window_ends_on"]
                .as_str()
                .and_then(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()),
        ) {
            if end < start {
                errors.push(format!(
                    "{label}.window_ends_on must follow window_starts_on"
                ));
            }
        }
    }
    errors
}

async fn authorized(
    db: &DatabaseTransaction,
    context: &AuthContext,
    plan: &ImportPlan,
    index: &ExistingIndex,
) -> Result<bool, ApiError> {
    if household_manager(context) {
        return Ok(true);
    }
    if !plan.rows("locations").is_empty() {
        return Ok(false);
    }
    let grants = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(context.membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(grant::Column::RevokedAt.is_null())
        .all(db)
        .await
        .map_err(database_error)?;
    let now = Utc::now().naive_utc();
    let manageable_ids: HashSet<i64> = grants
        .into_iter()
        .filter(|grant| {
            grant.access_level == "manage" && grant.expires_at.is_none_or(|expires| expires > now)
        })
        .map(|grant| grant.person_id)
        .collect();
    let mut referenced_people = HashSet::new();
    for kind in [
        "people",
        "schedules",
        "person_medications",
        "notification_preferences",
        "health_events",
    ] {
        for row in plan.rows(kind) {
            let field = if kind == "people" {
                "portable_id"
            } else {
                "person_portable_id"
            };
            if let Some(person) = row[field].as_str() {
                referenced_people.insert(person.to_owned());
            }
            if let Some(person) = row["portable_id"]
                .as_str()
                .and_then(|id| index.owner(kind, id))
            {
                referenced_people.insert(person.to_owned());
            }
        }
    }
    for kind in [
        "medication_takes",
        "medication_pause_periods",
        "dose_occurrences",
    ] {
        for row in plan.rows(kind) {
            if let Some(person) = row["portable_id"]
                .as_str()
                .and_then(|id| index.owner(kind, id))
            {
                referenced_people.insert(person.to_owned());
            }
            let source_kind = match row["source_type"].as_str() {
                Some("schedule") => "schedules",
                Some("person_medication") => "person_medications",
                _ => return Ok(false),
            };
            let Some(source_id) = row["source_portable_id"].as_str() else {
                return Ok(false);
            };
            if let Some(person) = index.owner(source_kind, source_id) {
                referenced_people.insert(person.to_owned());
            }
            if let Some(person) = plan
                .rows(source_kind)
                .iter()
                .find(|source| source["portable_id"] == source_id)
                .and_then(|source| source["person_portable_id"].as_str())
            {
                referenced_people.insert(person.to_owned());
            }
        }
    }
    if referenced_people.is_empty()
        || referenced_people.iter().any(|portable_id| {
            index
                .get("people", portable_id)
                .is_none_or(|id| !manageable_ids.contains(&id))
        })
    {
        return Ok(false);
    }
    let incoming_linked_medications: HashSet<String> = ["schedules", "person_medications"]
        .iter()
        .flat_map(|kind| {
            plan.rows(kind).iter().filter_map(|row| {
                let person_id = row["person_portable_id"].as_str()?;
                if !referenced_people.contains(person_id) {
                    return None;
                }
                row["medication_portable_id"].as_str().map(str::to_owned)
            })
        })
        .collect();
    let mut existing_authorized_medications = HashSet::new();
    for kind in ["schedules", "person_medications"] {
        if let (Some(owners), Some(medications)) =
            (index.owners.get(kind), index.source_medications.get(kind))
        {
            for (source_id, owner) in owners {
                if index
                    .get("people", owner)
                    .is_some_and(|id| manageable_ids.contains(&id))
                {
                    if let Some(medication) = medications.get(source_id) {
                        existing_authorized_medications.insert(medication.clone());
                    }
                }
            }
        }
    }
    let existing_medications: HashSet<String> = index
        .ids
        .get("medications")
        .map(|ids| ids.keys().cloned().collect())
        .unwrap_or_default();
    let planned_new_medications: HashSet<String> = plan
        .rows("medications")
        .iter()
        .filter_map(|row| row["portable_id"].as_str())
        .filter(|id| {
            !existing_medications.contains(*id) && incoming_linked_medications.contains(*id)
        })
        .map(str::to_owned)
        .collect();
    let target_allowed = |id: &str| {
        delegated_medication_target_allowed(
            id,
            &existing_authorized_medications,
            &planned_new_medications,
            &existing_medications,
        )
    };
    if ["schedules", "person_medications"].iter().any(|kind| {
        plan.rows(kind).iter().any(|row| {
            row["medication_portable_id"]
                .as_str()
                .is_none_or(|id| !target_allowed(id))
        })
    }) || plan.rows("medications").iter().any(|row| {
        row["portable_id"]
            .as_str()
            .is_none_or(|id| !target_allowed(id))
    }) || plan.rows("dosage_options").iter().any(|row| {
        let Some(target) = row["medication_portable_id"].as_str() else {
            return true;
        };
        let existing_parent = row["portable_id"]
            .as_str()
            .and_then(|id| index.dosage_medications.get(id).map(String::as_str));
        !delegated_dosage_write_allowed(
            existing_parent,
            target,
            &existing_authorized_medications,
            &planned_new_medications,
            &existing_medications,
        )
    }) {
        return Ok(false);
    }
    Ok(true)
}

fn delegated_medication_target_allowed(
    target: &str,
    existing_authorized: &HashSet<String>,
    planned_new: &HashSet<String>,
    existing: &HashSet<String>,
) -> bool {
    if existing.contains(target) {
        existing_authorized.contains(target)
    } else {
        planned_new.contains(target)
    }
}

fn delegated_dosage_write_allowed(
    current_parent: Option<&str>,
    target: &str,
    existing_authorized: &HashSet<String>,
    planned_new: &HashSet<String>,
    existing: &HashSet<String>,
) -> bool {
    current_parent.is_none_or(|parent| existing_authorized.contains(parent))
        && delegated_medication_target_allowed(target, existing_authorized, planned_new, existing)
}

fn reference(
    plan: &ImportPlan,
    index: &ExistingIndex,
    errors: &mut Vec<String>,
    target_field: (&str, usize, &str, &str, bool),
) {
    let (kind, row_index, field, target, required) = target_field;
    let row = &plan.rows(kind)[row_index];
    let value = row[field].as_str().filter(|value| !value.is_empty());
    match value {
        Some(value) if index.has(plan, target, value) => {}
        None if !required && row[field].is_null() => {}
        _ => errors.push(format!(
            "{kind}[{row_index}].{field} references unknown {target} portable ID"
        )),
    }
}

async fn preflight(
    db: &DatabaseTransaction,
    context: &AuthContext,
    plan: &ImportPlan,
    index: &ExistingIndex,
) -> Result<Preflight, ApiError> {
    let mut errors = plan.errors.clone();
    let mut conflicts = Vec::new();
    for kind in BASE_TYPES.iter().chain(V2_TYPES) {
        for (index, row) in plan.rows(kind).iter().enumerate() {
            errors.extend(validate_row(kind, index, row));
        }
    }
    for kind in ["locations", "people", "medications"] {
        let mut names = HashMap::new();
        let field = if kind == "people" { "email" } else { "name" };
        for row in plan.rows(kind) {
            let Some(value) = row[field].as_str().filter(|value| !value.is_empty()) else {
                continue;
            };
            let portable_id = row["portable_id"].as_str().unwrap_or_default();
            let lower = value.to_lowercase();
            let current = names.get(&lower).copied().or_else(|| {
                index
                    .names
                    .get(kind)
                    .and_then(|rows| rows.get(&lower).map(String::as_str))
            });
            if let Some(other) = current {
                if other != portable_id {
                    conflicts.push(json!({"record_type": kind, "portable_id": portable_id,
                        "field": field, "existing_portable_id": other}));
                }
            }
            names.insert(lower, portable_id);
        }
    }
    for (row_index, row) in plan.rows("people").iter().enumerate() {
        if row["person_type"]
            .as_str()
            .is_some_and(|kind| matches!(kind, "minor" | "dependent_adult"))
            && row["has_capacity"] == true
        {
            errors.push(format!(
                "people[{row_index}].has_capacity must be false for minors and dependent adults"
            ));
        }
    }
    for (i, row) in plan.rows("people").iter().enumerate() {
        if let Some(locations) = row["location_portable_ids"].as_array() {
            for location in locations {
                if location
                    .as_str()
                    .is_none_or(|id| !index.has(plan, "locations", id))
                {
                    errors.push(format!("people[{i}].location_portable_ids references unknown locations portable ID"));
                }
            }
        } else if !row["location_portable_ids"].is_null() {
            errors.push(format!(
                "people[{i}].location_portable_ids must be an array"
            ));
        }
    }
    for (kind, field, target, required) in [
        ("medications", "location_portable_id", "locations", true),
        (
            "dosage_options",
            "medication_portable_id",
            "medications",
            true,
        ),
        ("schedules", "person_portable_id", "people", true),
        ("schedules", "medication_portable_id", "medications", true),
        (
            "schedules",
            "source_dosage_option_portable_id",
            "dosage_options",
            false,
        ),
        ("person_medications", "person_portable_id", "people", true),
        (
            "person_medications",
            "medication_portable_id",
            "medications",
            true,
        ),
        (
            "person_medications",
            "source_dosage_option_portable_id",
            "dosage_options",
            false,
        ),
        (
            "medication_takes",
            "taken_from_medication_portable_id",
            "medications",
            false,
        ),
        (
            "medication_takes",
            "taken_from_location_portable_id",
            "locations",
            false,
        ),
        (
            "notification_preferences",
            "person_portable_id",
            "people",
            true,
        ),
        (
            "dose_occurrences",
            "medication_take_portable_id",
            "medication_takes",
            false,
        ),
        ("health_events", "person_portable_id", "people", true),
    ] {
        for i in 0..plan.rows(kind).len() {
            reference(plan, index, &mut errors, (kind, i, field, target, required));
        }
    }
    for kind in [
        "medication_takes",
        "medication_pause_periods",
        "dose_occurrences",
    ] {
        for (i, row) in plan.rows(kind).iter().enumerate() {
            let target = match row["source_type"].as_str() {
                Some("schedule") => "schedules",
                Some("person_medication") => "person_medications",
                _ => {
                    errors.push(format!("{kind}[{i}].source_type is unsupported"));
                    continue;
                }
            };
            reference(
                plan,
                index,
                &mut errors,
                (kind, i, "source_portable_id", target, true),
            );
        }
    }
    for (i, row) in plan.rows("health_events").iter().enumerate() {
        if let Some(medications) = row["medication_portable_ids"].as_array() {
            for medication in medications {
                if medication
                    .as_str()
                    .is_none_or(|id| !index.has(plan, "medications", id))
                {
                    errors.push(format!("health_events[{i}].medication_portable_ids references unknown medications portable ID"));
                }
            }
        } else if !row["medication_portable_ids"].is_null() {
            errors.push(format!(
                "health_events[{i}].medication_portable_ids must be an array"
            ));
        }
    }
    for kind in ["schedules", "person_medications"] {
        for (i, row) in plan.rows(kind).iter().enumerate() {
            let Some(medication_id) = row["medication_portable_id"].as_str() else {
                continue;
            };
            if let Some(dosage_id) = row["source_dosage_option_portable_id"].as_str() {
                if index.dosage_medication(plan, dosage_id) != Some(medication_id) {
                    errors.push(format!(
                        "{kind}[{i}].source_dosage_option_portable_id must belong to medication"
                    ));
                }
                if let Some((amount, unit)) = index.dosage_value(plan, dosage_id) {
                    if decimal(&row["dose_amount"]) != Some(amount)
                        || row["dose_unit"].as_str() != Some(unit)
                    {
                        errors.push(format!("{kind}[{i}] dose must match source dosage option"));
                    }
                }
            }
        }
    }
    for (i, row) in plan.rows("medication_takes").iter().enumerate() {
        let Some(source_type) = row["source_type"].as_str() else {
            continue;
        };
        let Some(source_id) = row["source_portable_id"].as_str() else {
            continue;
        };
        let source_kind = if source_type == "schedule" {
            "schedules"
        } else {
            "person_medications"
        };
        let Some(source_medication) = index.source_medication(plan, source_kind, source_id) else {
            continue;
        };
        if let Some(stock_medication) = row["taken_from_medication_portable_id"].as_str() {
            if stock_medication != source_medication {
                errors.push(format!("medication_takes[{i}].taken_from_medication_portable_id does not match source medication"));
            }
            if let Some(stock_location) = row["taken_from_location_portable_id"].as_str() {
                if index.medication_location(plan, stock_medication) != Some(stock_location) {
                    errors.push(format!("medication_takes[{i}].taken_from_location_portable_id does not match medication location"));
                }
            }
        }
    }
    for (i, row) in plan.rows("dose_occurrences").iter().enumerate() {
        if let Some(take_id) = row["medication_take_portable_id"].as_str() {
            if index.take_source(plan, take_id)
                != Some((
                    row["source_type"].as_str().unwrap_or_default(),
                    row["source_portable_id"].as_str().unwrap_or_default(),
                ))
            {
                errors.push(format!(
                    "dose_occurrences[{i}].medication_take_portable_id does not match source"
                ));
            }
        }
    }
    for kind in ["medication_takes", "dose_occurrences"] {
        for (i, row) in plan.rows(kind).iter().enumerate() {
            let Some(portable_id) = row["portable_id"].as_str() else {
                continue;
            };
            if index.get(kind, portable_id).is_none() {
                continue;
            }
            if let Ok(fields) = mapped_row(kind, row, index, context) {
                if let Err(message) = verify_immutable(db, kind, &fields).await {
                    errors.push(format!("{kind}[{i}] {message}"));
                }
            }
        }
    }
    let open_periods = db.query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT period.portable_id, CASE WHEN period.schedule_id IS NOT NULL THEN 'schedule' ELSE 'person_medication' END AS source_type, COALESCE(schedule.portable_id, assignment.portable_id) AS source_portable_id FROM medication_pause_periods period LEFT JOIN schedules schedule ON schedule.id = period.schedule_id AND schedule.household_id = period.household_id LEFT JOIN person_medications assignment ON assignment.id = period.person_medication_id AND assignment.household_id = period.household_id WHERE period.household_id = $1 AND period.ended_at IS NULL",
        [context.membership.household_id.into()])).await.map_err(database_error)?;
    let mut open_by_source = HashMap::new();
    for row in open_periods {
        let kind: String = row.try_get("", "source_type").map_err(database_error)?;
        let id: String = row
            .try_get("", "source_portable_id")
            .map_err(database_error)?;
        let period_id: String = row.try_get("", "portable_id").map_err(database_error)?;
        open_by_source.insert((kind, id), period_id);
    }
    for (i, row) in plan.rows("medication_pause_periods").iter().enumerate() {
        if row["ended_at"].is_null() {
            if let (Some(kind), Some(source), Some(portable_id)) = (
                row["source_type"].as_str(),
                row["source_portable_id"].as_str(),
                row["portable_id"].as_str(),
            ) {
                if open_by_source
                    .insert((kind.to_owned(), source.to_owned()), portable_id.to_owned())
                    .is_some_and(|existing| existing != portable_id)
                {
                    errors.push(format!(
                        "medication_pause_periods[{i}] source already has an open pause period"
                    ));
                }
            }
        }
        if let Some(portable_id) = row["portable_id"]
            .as_str()
            .filter(|id| index.get("medication_pause_periods", id).is_some())
        {
            if let Ok(fields) = mapped_row("medication_pause_periods", row, index, context) {
                if let Err(message) = verify_pause(db, context, portable_id, &fields).await {
                    errors.push(format!("medication_pause_periods[{i}] {message}"));
                }
            }
        }
    }
    if plan.v2 {
        for (kind, source_type) in [
            ("schedules", "schedule"),
            ("person_medications", "person_medication"),
        ] {
            for (i, row) in plan.rows(kind).iter().enumerate() {
                let has_open = open_by_source.contains_key(&(
                    source_type.to_owned(),
                    row["portable_id"].as_str().unwrap_or_default().to_owned(),
                ));
                if row["active"] == false && row["retired_at"].is_null() && !has_open {
                    errors.push(format!(
                        "{kind}[{i}] inactive source requires an open pause period"
                    ));
                }
            }
        }
    }
    Ok(Preflight {
        counts: plan.counts.clone(),
        conflicts,
        errors,
    })
}

fn text(row: &Value, field: &str) -> Result<String, String> {
    row[field]
        .as_str()
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| format!("{field} is required"))
}

fn foreign(index: &ExistingIndex, kind: &str, row: &Value, field: &str) -> Result<Value, String> {
    match row[field].as_str() {
        Some(portable_id) => index
            .get(kind, portable_id)
            .map(|id| json!(id))
            .ok_or_else(|| format!("{field} references unknown {kind} portable ID")),
        None if row[field].is_null() => Ok(Value::Null),
        _ => Err(format!("{field} must be a portable ID")),
    }
}

fn enum_value(
    row: &Value,
    field: &str,
    variants: &[&str],
    default: Option<&str>,
) -> Result<Value, String> {
    let selected = row[field]
        .as_str()
        .or(default)
        .ok_or_else(|| format!("{field} is required"))?;
    variants
        .iter()
        .position(|candidate| candidate == &selected)
        .map(|value| json!(value))
        .ok_or_else(|| format!("{field} is unsupported"))
}

fn copy_fields(row: &Value, fields: &[&str]) -> Map<String, Value> {
    fields
        .iter()
        .map(|field| ((*field).to_owned(), row[*field].clone()))
        .collect()
}

fn required_id(
    index: &ExistingIndex,
    kind: &str,
    row: &Value,
    field: &str,
) -> Result<Value, String> {
    let result = foreign(index, kind, row, field)?;
    if result.is_null() {
        Err(format!("{field} is required"))
    } else {
        Ok(result)
    }
}

fn mapped_row(
    kind: &str,
    row: &Value,
    index: &ExistingIndex,
    context: &AuthContext,
) -> Result<Map<String, Value>, String> {
    let mut fields = match kind {
        "locations" => copy_fields(row, &["name", "description"]),
        "people" => {
            let mut fields = copy_fields(row, &["name", "email", "date_of_birth", "has_capacity"]);
            fields.insert(
                "person_type".to_owned(),
                enum_value(
                    row,
                    "person_type",
                    &["adult", "minor", "dependent_adult"],
                    None,
                )?,
            );
            fields
        }
        "medications" => {
            let mut fields = copy_fields(
                row,
                &[
                    "name",
                    "friendly_name",
                    "category",
                    "description",
                    "dose_amount",
                    "dose_unit",
                    "current_supply",
                    "reorder_threshold",
                    "barcode",
                    "dmd_code",
                    "dmd_system",
                    "dmd_concept_class",
                ],
            );
            fields.insert(
                "location_id".to_owned(),
                required_id(index, "locations", row, "location_portable_id")?,
            );
            fields.insert(
                "default_schedule_type".to_owned(),
                enum_value(
                    row,
                    "default_schedule_type",
                    &[
                        "daily",
                        "multiple_daily",
                        "weekly",
                        "specific_dates",
                        "prn",
                        "tapering",
                        "every_other_day",
                    ],
                    Some("multiple_daily"),
                )?,
            );
            fields
        }
        "dosage_options" => {
            let mut fields = copy_fields(
                row,
                &[
                    "amount",
                    "unit",
                    "frequency",
                    "description",
                    "default_for_adults",
                    "default_for_children",
                    "default_max_daily_doses",
                    "default_min_hours_between_doses",
                    "current_supply",
                    "reorder_threshold",
                ],
            );
            fields.insert(
                "medication_id".to_owned(),
                required_id(index, "medications", row, "medication_portable_id")?,
            );
            fields.insert(
                "default_dose_cycle".to_owned(),
                enum_value(
                    row,
                    "default_dose_cycle",
                    &["daily", "weekly", "monthly"],
                    Some("daily"),
                )?,
            );
            fields
        }
        "schedules" => {
            let mut fields = copy_fields(
                row,
                &[
                    "dose_amount",
                    "dose_unit",
                    "frequency",
                    "max_daily_doses",
                    "min_hours_between_doses",
                    "schedule_config",
                    "start_date",
                    "end_date",
                    "active",
                    "notes",
                    "retired_at",
                ],
            );
            fields.insert(
                "person_id".to_owned(),
                required_id(index, "people", row, "person_portable_id")?,
            );
            fields.insert(
                "medication_id".to_owned(),
                required_id(index, "medications", row, "medication_portable_id")?,
            );
            fields.insert(
                "source_dosage_option_id".to_owned(),
                foreign(
                    index,
                    "dosage_options",
                    row,
                    "source_dosage_option_portable_id",
                )?,
            );
            fields.insert(
                "dose_cycle".to_owned(),
                if row["dose_cycle"].is_null() {
                    Value::Null
                } else {
                    enum_value(row, "dose_cycle", &["daily", "weekly", "monthly"], None)?
                },
            );
            fields.insert(
                "schedule_type".to_owned(),
                enum_value(
                    row,
                    "schedule_type",
                    &[
                        "daily",
                        "multiple_daily",
                        "weekly",
                        "specific_dates",
                        "prn",
                        "tapering",
                        "every_other_day",
                    ],
                    Some("daily"),
                )?,
            );
            fields
        }
        "person_medications" => {
            let mut fields = copy_fields(
                row,
                &[
                    "dose_amount",
                    "dose_unit",
                    "max_daily_doses",
                    "min_hours_between_doses",
                    "active",
                    "notes",
                    "position",
                    "retired_at",
                ],
            );
            fields.insert(
                "person_id".to_owned(),
                required_id(index, "people", row, "person_portable_id")?,
            );
            fields.insert(
                "medication_id".to_owned(),
                required_id(index, "medications", row, "medication_portable_id")?,
            );
            fields.insert(
                "source_dosage_option_id".to_owned(),
                foreign(
                    index,
                    "dosage_options",
                    row,
                    "source_dosage_option_portable_id",
                )?,
            );
            fields.insert(
                "dose_cycle".to_owned(),
                if row["dose_cycle"].is_null() {
                    Value::Null
                } else {
                    enum_value(row, "dose_cycle", &["daily", "weekly", "monthly"], None)?
                },
            );
            fields.insert(
                "administration_kind".to_owned(),
                enum_value(
                    row,
                    "administration_kind",
                    &["routine", "as_needed"],
                    Some("as_needed"),
                )?,
            );
            fields
        }
        "medication_takes" => {
            let mut fields = copy_fields(
                row,
                &["client_uuid", "taken_at", "dose_amount", "dose_unit"],
            );
            let (schedule_id, assignment_id) = source_ids(index, row)?;
            fields.insert("schedule_id".to_owned(), schedule_id);
            fields.insert("person_medication_id".to_owned(), assignment_id);
            fields.insert(
                "taken_from_medication_id".to_owned(),
                foreign(
                    index,
                    "medications",
                    row,
                    "taken_from_medication_portable_id",
                )?,
            );
            fields.insert(
                "taken_from_location_id".to_owned(),
                foreign(index, "locations", row, "taken_from_location_portable_id")?,
            );
            fields
        }
        "notification_preferences" => {
            let mut fields = copy_fields(
                row,
                &[
                    "enabled",
                    "dose_due_enabled",
                    "missed_dose_enabled",
                    "low_stock_enabled",
                    "private_text_enabled",
                    "morning_time",
                    "afternoon_time",
                    "evening_time",
                    "night_time",
                ],
            );
            fields.insert(
                "person_id".to_owned(),
                required_id(index, "people", row, "person_portable_id")?,
            );
            fields
        }
        "medication_pause_periods" => {
            let mut fields = copy_fields(
                row,
                &["reason", "note", "started_at", "ended_at", "legacy_context"],
            );
            if !row["created_at"].is_null() {
                fields.insert("created_at".to_owned(), row["created_at"].clone());
            }
            let (schedule_id, assignment_id) = source_ids(index, row)?;
            fields.insert("schedule_id".to_owned(), schedule_id);
            fields.insert("person_medication_id".to_owned(), assignment_id);
            fields.insert("imported_context".to_owned(), json!(true));
            fields.insert(
                "imported_actor_references".to_owned(),
                json!({
                    "recorded_by_person_portable_id": row["recorded_by_person_portable_id"],
                    "resumed_by_person_portable_id": row["resumed_by_person_portable_id"],
                }),
            );
            fields
        }
        "dose_occurrences" => {
            let mut fields = copy_fields(
                row,
                &[
                    "window_starts_on",
                    "window_ends_on",
                    "position",
                    "scheduled_at",
                    "outcome",
                    "reason",
                    "note",
                    "resolved_at",
                ],
            );
            let (schedule_id, assignment_id) = source_ids(index, row)?;
            fields.insert("schedule_id".to_owned(), schedule_id);
            fields.insert("person_medication_id".to_owned(), assignment_id);
            fields.insert(
                "medication_take_id".to_owned(),
                foreign(
                    index,
                    "medication_takes",
                    row,
                    "medication_take_portable_id",
                )?,
            );
            fields.insert(
                "resolved_by_membership_id".to_owned(),
                if row["outcome"] == "open" {
                    Value::Null
                } else {
                    json!(context.membership.id)
                },
            );
            fields
        }
        "health_events" => {
            let mut fields = copy_fields(row, &["title", "notes", "started_on", "ended_on"]);
            fields.insert(
                "person_id".to_owned(),
                required_id(index, "people", row, "person_portable_id")?,
            );
            fields.insert(
                "event_kind".to_owned(),
                enum_value(
                    row,
                    "event_kind",
                    &["illness", "suspected_side_effect"],
                    None,
                )?,
            );
            fields.insert(
                "severity".to_owned(),
                if row["severity"].is_null() {
                    Value::Null
                } else {
                    enum_value(row, "severity", &["mild", "moderate", "severe"], None)?
                },
            );
            fields
        }
        _ => return Err("Unsupported portable record type".to_owned()),
    };
    fields.insert(
        "household_id".to_owned(),
        json!(context.membership.household_id),
    );
    fields.insert("portable_id".to_owned(), row["portable_id"].clone());
    Ok(fields)
}

fn source_ids(index: &ExistingIndex, row: &Value) -> Result<(Value, Value), String> {
    let portable_id = row["source_portable_id"]
        .as_str()
        .ok_or("source_portable_id is required")?;
    match row["source_type"].as_str() {
        Some("schedule") => Ok((
            json!(index
                .get("schedules", portable_id)
                .ok_or("Unknown schedule source")?),
            Value::Null,
        )),
        Some("person_medication") => Ok((
            Value::Null,
            json!(index
                .get("person_medications", portable_id)
                .ok_or("Unknown person medication source")?),
        )),
        _ => Err("Unsupported medication source type".to_owned()),
    }
}

fn record_person<'a>(
    kind: &str,
    row: &'a Value,
    plan: &'a ImportPlan,
    index: &'a ExistingIndex,
) -> Option<&'a str> {
    if kind == "people" {
        return row["portable_id"].as_str();
    }
    if let Some(person) = row["person_portable_id"].as_str() {
        return Some(person);
    }
    if matches!(
        kind,
        "medication_takes" | "medication_pause_periods" | "dose_occurrences"
    ) {
        let source_type = row["source_type"].as_str()?;
        let source_id = row["source_portable_id"].as_str()?;
        let source_kind = if source_type == "schedule" {
            "schedules"
        } else {
            "person_medications"
        };
        return plan
            .rows(source_kind)
            .iter()
            .find(|source| source["portable_id"] == source_id)
            .and_then(|source| source["person_portable_id"].as_str())
            .or_else(|| index.owner(source_kind, source_id));
    }
    None
}

async fn upsert(
    db: &DatabaseTransaction,
    kind: &str,
    fields: &Map<String, Value>,
) -> Result<Option<i64>, String> {
    let table = table(kind);
    let columns: Vec<&str> = fields.keys().map(String::as_str).collect();
    let names = columns.join(", ");
    let updates = columns
        .iter()
        .filter(|column| !matches!(**column, "household_id" | "portable_id" | "created_at"))
        .map(|column| format!("{column} = EXCLUDED.{column}"))
        .collect::<Vec<_>>()
        .join(", ");
    let changed = columns
        .iter()
        .filter(|column| !matches!(**column, "household_id" | "portable_id" | "created_at"))
        .map(|column| format!("{table}.{column} IS DISTINCT FROM EXCLUDED.{column}"))
        .collect::<Vec<_>>()
        .join(" OR ");
    let conflict = format!("DO UPDATE SET {updates}, updated_at = NOW() WHERE {changed}");
    let created_column = if fields.contains_key("created_at") {
        ""
    } else {
        ", created_at"
    };
    let created_value = if fields.contains_key("created_at") {
        ""
    } else {
        ", NOW()"
    };
    let sql = format!("INSERT INTO {table} ({names}{created_column}, updated_at) SELECT {names}{created_value}, NOW() FROM jsonb_populate_record(NULL::{table}, $1::jsonb) ON CONFLICT (household_id, portable_id) {conflict} RETURNING id");
    let payload = Value::Object(fields.clone()).to_string();
    let result = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            [payload.into()],
        ))
        .await
        .map_err(|_| format!("{kind} could not be imported"))?;
    result
        .map(|row| {
            row.try_get("", "id")
                .map_err(|_| format!("{kind} could not be imported"))
        })
        .transpose()
}

async fn snapshot(
    db: &DatabaseTransaction,
    kind: &str,
    household_id: i64,
    portable_id: &str,
) -> Result<Option<Value>, String> {
    let sql = format!("SELECT to_jsonb(record) AS value FROM {} AS record WHERE household_id = $1 AND portable_id = $2", table(kind));
    db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        [household_id.into(), portable_id.into()],
    ))
    .await
    .map_err(|_| format!("{kind} could not be checked"))?
    .map(|row| {
        row.try_get("", "value")
            .map_err(|_| format!("{kind} could not be checked"))
    })
    .transpose()
}

async fn write_records(
    db: &DatabaseTransaction,
    context: &AuthContext,
    plan: &ImportPlan,
    index: &mut ExistingIndex,
    request_id: &str,
) -> Result<(), String> {
    for kind in [
        "locations",
        "people",
        "medications",
        "dosage_options",
        "schedules",
        "person_medications",
        "medication_pause_periods",
        "medication_takes",
        "notification_preferences",
        "dose_occurrences",
        "health_events",
    ] {
        let mut rows: Vec<&Value> = plan.rows(kind).iter().collect();
        if kind == "medication_pause_periods" {
            rows.sort_by_key(|row| row["ended_at"].is_null());
        }
        for row in rows {
            let portable_id = text(row, "portable_id")?;
            let existed = index.get(kind, &portable_id).is_some();
            let before = if existed {
                snapshot(db, kind, context.membership.household_id, &portable_id).await?
            } else {
                None
            };
            let fields = mapped_row(kind, row, index, context)?;
            if existed && matches!(kind, "medication_takes" | "dose_occurrences") {
                verify_immutable(db, kind, &fields).await?;
                continue;
            }
            let id = if existed && kind == "medication_pause_periods" {
                if verify_pause(db, context, &portable_id, &fields).await? {
                    resume_pause(db, context, &portable_id, &fields["ended_at"]).await?;
                }
                index
                    .get(kind, &portable_id)
                    .ok_or_else(|| "Imported pause is unavailable".to_owned())?
            } else {
                upsert(db, kind, &fields)
                    .await?
                    .or_else(|| index.get(kind, &portable_id))
                    .ok_or_else(|| format!("{kind} could not be imported"))?
            };
            index.put(kind, portable_id.clone(), id);
            if kind == "people" {
                grant_importer_access(db, context, id).await?;
                write_location_memberships(db, context, row, index, id).await?;
            }
            if kind == "health_events" {
                write_health_links(db, context, row, index, id).await?;
            }
            let after = snapshot(db, kind, context.membership.household_id, &portable_id).await?;
            if before == after {
                continue;
            }
            let record_type = match kind {
                "people" => "Person",
                "locations" => "Location",
                "medications" => "Medication",
                "dosage_options" => "MedicationDosageOption",
                "schedules" => "Schedule",
                "person_medications" => "PersonMedication",
                "medication_pause_periods" => "MedicationPausePeriod",
                "medication_takes" => "MedicationTake",
                "notification_preferences" => "NotificationPreference",
                "dose_occurrences" => "MedicationDoseOccurrence",
                "health_events" => "HealthEvent",
                _ => unreachable!(),
            };
            record_version(
                db,
                context,
                request_id,
                record_type,
                id,
                if existed { "update" } else { "create" },
                before,
                after,
            )
            .await
            .map_err(|_| format!("{kind} version could not be recorded"))?;
            record_change(
                db,
                context,
                request_id,
                SyncRecord {
                    record_type,
                    record_id: id,
                    portable_id: &portable_id,
                    action: if existed { "update" } else { "create" },
                    person_portable_id: record_person(kind, row, plan, index),
                },
            )
            .await
            .map_err(|_| format!("{kind} change could not be recorded"))?;
        }
        if kind == "medication_pause_periods" && plan.v2 {
            reconcile_pauses(db, context, plan, index, request_id).await?;
        }
    }
    Ok(())
}

async fn verify_immutable(
    db: &DatabaseTransaction,
    kind: &str,
    fields: &Map<String, Value>,
) -> Result<(), String> {
    let table = table(kind);
    let columns = if kind == "medication_takes" {
        &[
            "client_uuid",
            "schedule_id",
            "person_medication_id",
            "taken_at",
            "dose_amount",
            "dose_unit",
            "taken_from_medication_id",
            "taken_from_location_id",
        ][..]
    } else {
        &[
            "schedule_id",
            "person_medication_id",
            "window_starts_on",
            "window_ends_on",
            "position",
            "scheduled_at",
            "outcome",
            "reason",
            "note",
            "resolved_at",
            "medication_take_id",
        ][..]
    };
    let comparison = columns
        .iter()
        .map(|column| format!("existing.{column} IS NOT DISTINCT FROM incoming.{column}"))
        .collect::<Vec<_>>()
        .join(" AND ");
    let sql = format!("SELECT ({comparison}) AS same FROM {table} AS existing CROSS JOIN jsonb_populate_record(NULL::{table}, $1::jsonb) AS incoming WHERE existing.household_id = $2 AND existing.portable_id = $3");
    let household_id = fields["household_id"]
        .as_i64()
        .ok_or("household_id is invalid")?;
    let portable_id = fields["portable_id"]
        .as_str()
        .ok_or("portable_id is invalid")?;
    let payload = Value::Object(fields.clone()).to_string();
    let existing = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            [payload.into(), household_id.into(), portable_id.into()],
        ))
        .await
        .map_err(|_| format!("{kind} could not be checked"))?
        .ok_or_else(|| format!("{kind} is unavailable"))?;
    let same: bool = existing
        .try_get("", "same")
        .map_err(|_| format!("{kind} could not be checked"))?;
    if !same {
        return Err(format!("{kind} conflicts with existing history"));
    }
    Ok(())
}

async fn verify_pause(
    db: &DatabaseTransaction,
    context: &AuthContext,
    portable_id: &str,
    fields: &Map<String, Value>,
) -> Result<bool, String> {
    let sql = "SELECT period.imported_context, period.ended_at IS NULL AS is_open, (period.schedule_id IS NOT DISTINCT FROM incoming.schedule_id AND period.person_medication_id IS NOT DISTINCT FROM incoming.person_medication_id AND period.reason IS NOT DISTINCT FROM incoming.reason AND period.note IS NOT DISTINCT FROM incoming.note AND period.started_at IS NOT DISTINCT FROM incoming.started_at AND period.legacy_context IS NOT DISTINCT FROM incoming.legacy_context AND (period.ended_at IS NOT DISTINCT FROM incoming.ended_at OR (period.imported_context AND period.ended_at IS NULL)) AND COALESCE(recorded_person.portable_id, period.imported_actor_references->>'recorded_by_person_portable_id') IS NOT DISTINCT FROM incoming.imported_actor_references->>'recorded_by_person_portable_id' AND COALESCE(resumed_person.portable_id, period.imported_actor_references->>'resumed_by_person_portable_id') IS NOT DISTINCT FROM incoming.imported_actor_references->>'resumed_by_person_portable_id') AS same FROM medication_pause_periods period CROSS JOIN jsonb_populate_record(NULL::medication_pause_periods, $1::jsonb) incoming LEFT JOIN household_memberships recorded ON recorded.id = period.recorded_by_membership_id LEFT JOIN people recorded_person ON recorded_person.id = recorded.person_id LEFT JOIN household_memberships resumed ON resumed.id = period.resumed_by_membership_id LEFT JOIN people resumed_person ON resumed_person.id = resumed.person_id WHERE period.household_id = $2 AND period.portable_id = $3";
    let payload = Value::Object(fields.clone()).to_string();
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            [
                payload.into(),
                context.membership.household_id.into(),
                portable_id.into(),
            ],
        ))
        .await
        .map_err(|_| "Imported pause could not be checked".to_owned())?
        .ok_or_else(|| "Imported pause is unavailable".to_owned())?;
    let same: bool = row
        .try_get("", "same")
        .map_err(|_| "Imported pause could not be checked")?;
    if !same {
        return Err("conflicts with recorded history or actors".to_owned());
    }
    let imported: bool = row
        .try_get("", "imported_context")
        .map_err(|_| "Imported pause could not be checked")?;
    let is_open: bool = row
        .try_get("", "is_open")
        .map_err(|_| "Imported pause could not be checked")?;
    Ok(imported && is_open && !fields["ended_at"].is_null())
}

async fn resume_pause(
    db: &DatabaseTransaction,
    context: &AuthContext,
    portable_id: &str,
    ended_at: &Value,
) -> Result<(), String> {
    let ended = ended_at.as_str().ok_or("Pause end time is invalid")?;
    db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE medication_pause_periods SET ended_at = $1::timestamptz, updated_at = NOW() WHERE household_id = $2 AND portable_id = $3 AND imported_context AND ended_at IS NULL",
        [ended.into(), context.membership.household_id.into(), portable_id.into()]))
        .await.map_err(|_| "Imported pause could not be resumed".to_owned())?;
    Ok(())
}

async fn reconcile_pauses(
    db: &DatabaseTransaction,
    context: &AuthContext,
    plan: &ImportPlan,
    index: &ExistingIndex,
    request_id: &str,
) -> Result<(), String> {
    for (kind, source_type, foreign_key, record_type) in [
        ("schedules", "schedule", "schedule_id", "Schedule"),
        (
            "person_medications",
            "person_medication",
            "person_medication_id",
            "PersonMedication",
        ),
    ] {
        let mut portable_ids: HashSet<String> = plan
            .rows(kind)
            .iter()
            .filter_map(|row| row["portable_id"].as_str().map(str::to_owned))
            .collect();
        portable_ids.extend(
            plan.rows("medication_pause_periods")
                .iter()
                .filter_map(|row| {
                    if row["source_type"] == source_type {
                        row["source_portable_id"].as_str().map(str::to_owned)
                    } else {
                        None
                    }
                }),
        );
        if portable_ids.is_empty() {
            continue;
        }
        let ids = json!(portable_ids.into_iter().collect::<Vec<_>>()).to_string();
        let table = table(kind);
        let sql = format!("WITH desired AS (SELECT source.id, EXISTS (SELECT 1 FROM medication_pause_periods period WHERE period.household_id = source.household_id AND period.{foreign_key} = source.id AND period.ended_at IS NULL) AS paused FROM {table} source WHERE source.household_id = $1 AND source.portable_id IN (SELECT jsonb_array_elements_text($2::jsonb))) UPDATE {table} source SET active = NOT desired.paused, updated_at = NOW() FROM desired WHERE source.id = desired.id AND source.active IS DISTINCT FROM NOT desired.paused AND (source.retired_at IS NULL OR desired.paused) RETURNING source.id, source.portable_id, source.active");
        let changed = db
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                sql,
                [context.membership.household_id.into(), ids.into()],
            ))
            .await
            .map_err(|_| format!("{kind} pause state could not be restored"))?;
        for row in changed {
            let id: i64 = row
                .try_get("", "id")
                .map_err(|_| format!("{kind} pause state could not be restored"))?;
            let portable_id: String = row
                .try_get("", "portable_id")
                .map_err(|_| format!("{kind} pause state could not be restored"))?;
            let active: bool = row
                .try_get("", "active")
                .map_err(|_| format!("{kind} pause state could not be restored"))?;
            record_version(
                db,
                context,
                request_id,
                record_type,
                id,
                "update",
                Some(json!({"active": !active})),
                Some(json!({"active": active})),
            )
            .await
            .map_err(|_| format!("{kind} pause version could not be recorded"))?;
            let person = plan
                .rows(kind)
                .iter()
                .find(|source| source["portable_id"] == portable_id)
                .and_then(|source| source["person_portable_id"].as_str())
                .or_else(|| index.owner(kind, &portable_id));
            record_change(
                db,
                context,
                request_id,
                SyncRecord {
                    record_type,
                    record_id: id,
                    portable_id: &portable_id,
                    action: "update",
                    person_portable_id: person,
                },
            )
            .await
            .map_err(|_| format!("{kind} pause change could not be recorded"))?;
        }
    }
    Ok(())
}

async fn grant_importer_access(
    db: &DatabaseTransaction,
    context: &AuthContext,
    person_id: i64,
) -> Result<(), String> {
    let sql = "SELECT id, access_level, expires_at, carer_relationship_id FROM person_access_grants WHERE household_id = $1 AND household_membership_id = $2 AND person_id = $3 AND revoked_at IS NULL FOR UPDATE";
    let existing = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            [
                context.membership.household_id.into(),
                context.membership.id.into(),
                person_id.into(),
            ],
        ))
        .await
        .map_err(|_| "Imported person access could not be checked".to_owned())?;
    let changed = if let Some(existing) = existing {
        let id: i64 = existing
            .try_get("", "id")
            .map_err(|_| "Imported person access could not be checked")?;
        let carer_id: Option<i64> = existing
            .try_get("", "carer_relationship_id")
            .map_err(|_| "Imported person access could not be checked")?;
        if carer_id.is_some() {
            let access: String = existing
                .try_get("", "access_level")
                .map_err(|_| "Imported person access could not be checked")?;
            let expires: Option<chrono::NaiveDateTime> = existing
                .try_get("", "expires_at")
                .map_err(|_| "Imported person access could not be checked")?;
            if access != "manage" || expires.is_some() {
                return Err(
                    "relationship-owned access grant conflicts with imported manage access"
                        .to_owned(),
                );
            }
            return Ok(());
        }
        db.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE person_access_grants SET access_level = 'manage', updated_at = NOW() WHERE id = $1 AND access_level <> 'manage' RETURNING id", [id.into()]))
            .await.map_err(|_| "Imported person access could not be updated".to_owned())?
            .is_some()
    } else {
        db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO person_access_grants (household_id, household_membership_id, person_id, access_level, relationship_type, granted_by_membership_id, created_at, updated_at) VALUES ($1, $2, $3, 'manage', 'family_member', $2, NOW(), NOW())",
            [context.membership.household_id.into(), context.membership.id.into(), person_id.into()]))
            .await.map_err(|_| "Imported person access could not be granted".to_owned())?;
        true
    };
    if changed {
        db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE household_memberships SET permissions_version = permissions_version + 1, updated_at = NOW() WHERE id = $1 AND household_id = $2",
            [context.membership.id.into(), context.membership.household_id.into()]))
            .await.map_err(|_| "Imported person access could not be versioned".to_owned())?;
    }
    Ok(())
}

async fn write_location_memberships(
    db: &DatabaseTransaction,
    context: &AuthContext,
    row: &Value,
    index: &ExistingIndex,
    person_id: i64,
) -> Result<(), String> {
    let Some(locations) = row["location_portable_ids"].as_array() else {
        return Ok(());
    };
    for location in locations {
        let portable_id = location.as_str().ok_or("Location portable ID is invalid")?;
        let location_id = index
            .get("locations", portable_id)
            .ok_or("Imported location is unavailable")?;
        db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO location_memberships (household_id, person_id, location_id, created_at, updated_at) VALUES ($1, $2, $3, NOW(), NOW()) ON CONFLICT (person_id, location_id) DO NOTHING",
            [context.membership.household_id.into(), person_id.into(), location_id.into()]))
            .await.map_err(|_| "Imported location membership could not be created".to_owned())?;
    }
    Ok(())
}

async fn write_health_links(
    db: &DatabaseTransaction,
    context: &AuthContext,
    row: &Value,
    index: &ExistingIndex,
    event_id: i64,
) -> Result<(), String> {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "DELETE FROM health_event_medications WHERE household_id = $1 AND health_event_id = $2",
        [context.membership.household_id.into(), event_id.into()],
    ))
    .await
    .map_err(|_| "Health event medications could not be updated".to_owned())?;
    let Some(medications) = row["medication_portable_ids"].as_array() else {
        return Ok(());
    };
    for medication in medications {
        let portable_id = medication
            .as_str()
            .ok_or("Health event medication portable ID is invalid")?;
        let medication_id = index
            .get("medications", portable_id)
            .ok_or("Health event medication is unavailable")?;
        db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO health_event_medications (household_id, health_event_id, medication_id, medication_name, created_at, updated_at) SELECT $1, $2, id, COALESCE(name, friendly_name, 'Medication'), NOW(), NOW() FROM medications WHERE household_id = $1 AND id = $3",
            [context.membership.household_id.into(), event_id.into(), medication_id.into()]))
            .await.map_err(|_| "Health event medication could not be restored".to_owned())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{delegated_dosage_write_allowed, delegated_medication_target_allowed};
    use std::collections::HashSet;

    #[test]
    fn delegated_import_cannot_bootstrap_existing_medication_or_reparent_dosage() {
        let authorized: HashSet<String> = ["managed-med".to_owned()].into();
        let planned_new: HashSet<String> = ["new-med".to_owned()].into();
        let existing: HashSet<String> = ["managed-med".to_owned(), "other-med".to_owned()].into();

        assert!(!delegated_medication_target_allowed(
            "other-med",
            &authorized,
            &planned_new,
            &existing
        ));
        assert!(delegated_medication_target_allowed(
            "managed-med",
            &authorized,
            &planned_new,
            &existing
        ));
        assert!(delegated_medication_target_allowed(
            "new-med",
            &authorized,
            &planned_new,
            &existing
        ));
        assert!(!delegated_medication_target_allowed(
            "unplanned-new-med",
            &authorized,
            &planned_new,
            &existing
        ));
        assert!(!delegated_medication_target_allowed(
            "managed-med",
            &HashSet::new(),
            &planned_new,
            &existing
        ));
        assert!(!delegated_dosage_write_allowed(
            Some("other-med"),
            "managed-med",
            &authorized,
            &planned_new,
            &existing
        ));
        assert!(delegated_dosage_write_allowed(
            Some("managed-med"),
            "managed-med",
            &authorized,
            &planned_new,
            &existing
        ));
        assert!(!delegated_dosage_write_allowed(
            None,
            "other-med",
            &authorized,
            &planned_new,
            &existing
        ));
        assert!(delegated_dosage_write_allowed(
            None,
            "new-med",
            &authorized,
            &planned_new,
            &existing
        ));
    }
}
