use super::*;

pub(super) fn decimal(value: &Value) -> Option<Decimal> {
    match value {
        Value::Number(number) => Decimal::from_str(&number.to_string()).ok(),
        Value::String(text) => Decimal::from_str(text).ok(),
        _ => None,
    }
}

pub(super) fn validate_row(kind: &str, index: usize, row: &Value) -> Vec<String> {
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
