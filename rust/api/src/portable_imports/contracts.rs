use super::*;

pub(super) const BASE_TYPES: &[&str] = &[
    "people",
    "locations",
    "medications",
    "dosage_options",
    "schedules",
    "person_medications",
    "medication_takes",
    "notification_preferences",
];

pub(super) const V2_TYPES: &[&str] = &[
    "medication_pause_periods",
    "dose_occurrences",
    "health_events",
];

pub(super) struct ImportPlan {
    records: Map<String, Value>,
    pub(super) counts: Value,
    pub(super) errors: Vec<String>,
    pub(super) v2: bool,
}

impl ImportPlan {
    pub(super) fn parse(payload: Value) -> Result<Self, String> {
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

    pub(super) fn rows(&self, kind: &str) -> &[Value] {
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
