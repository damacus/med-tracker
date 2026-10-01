use super::*;
use sea_orm::prelude::Decimal;

fn apply_dose(context: &Context, draft: &mut TreatmentDraft) -> Result<(), Errors> {
    let medication = context
        .medications
        .iter()
        .find(|row| {
            numeric(row, "id").map(|id| id.to_string()).as_deref()
                == Some(draft.value("medication_id"))
        })
        .ok_or_else(|| payload::invalid("medication_id"))?;
    let mut options = context
        .options
        .iter()
        .filter(|row| numeric(row, "medication_id") == numeric(medication, "id"))
        .collect::<Vec<_>>();
    options.sort_by_key(|row| {
        (
            Decimal::from_str_exact(field(row, "amount")).unwrap_or_default(),
            numeric(row, "id"),
        )
    });
    let selected = draft.value("source_dosage_option_id");
    let option = if selected.is_empty() {
        let child = field(&context.person, "person_type") != "adult";
        options
            .iter()
            .copied()
            .find(|row| child && row["default_for_children"].as_bool() == Some(true))
            .or_else(|| {
                options
                    .iter()
                    .copied()
                    .find(|row| row["default_for_adults"].as_bool() == Some(true))
            })
            .or_else(|| options.first().copied())
    } else {
        Some(
            options
                .iter()
                .copied()
                .find(|row| {
                    numeric(row, "id").map(|id| id.to_string()).as_deref() == Some(selected)
                })
                .ok_or_else(|| payload::invalid("source_dosage_option_id"))?,
        )
    };
    if let Some(option) = option {
        for (name, key) in [
            ("dose_amount", "amount"),
            ("dose_unit", "unit"),
            ("dose_cycle", "default_dose_cycle"),
        ] {
            draft.fields.insert(name.into(), field(option, key).into());
        }
        draft.fields.insert(
            "source_dosage_option_id".into(),
            numeric(option, "id")
                .ok_or_else(|| payload::invalid("source_dosage_option_id"))?
                .to_string(),
        );
        if let Some(max) = option["default_max_daily_doses"].as_i64() {
            draft
                .fields
                .insert("max_daily_doses".into(), max.to_string());
        }
        let interval = field(option, "default_min_hours_between_doses");
        draft.fields.insert(
            "min_hours_between_doses".into(),
            if Decimal::from_str_exact(interval).ok() == Some(Decimal::ZERO) {
                String::new()
            } else {
                interval.into()
            },
        );
    } else {
        draft.fields.insert(
            "dose_amount".into(),
            field(medication, "dose_amount").into(),
        );
        draft
            .fields
            .insert("dose_unit".into(), field(medication, "dose_unit").into());
    }
    Ok(())
}

pub(super) fn apply(
    context: &Context,
    draft: &mut TreatmentDraft,
    intent: &str,
) -> Result<(), Errors> {
    if intent == "apply_dose" {
        return apply_dose(context, draft);
    }
    if intent == "change_type" {
        return Ok(());
    }
    if let Some(prefix) = intent.strip_prefix("add_") {
        let row_prefix = match prefix {
            "time" => "time_".to_owned(),
            "date" => "date_".to_owned(),
            "step" => "step_".to_owned(),
            value if value.starts_with("step_") && value.ends_with("_time") => format!("{value}_"),
            _ => return Err(payload::invalid("base")),
        };
        let initial = if prefix == "step"
            || prefix == "date"
            || (prefix == "time" && draft.value("schedule_type") == "multiple_daily")
        {
            2
        } else {
            1
        };
        let indices = draft.indices(&row_prefix, initial);
        for index in &indices {
            draft
                .fields
                .entry(if prefix == "step" {
                    format!("{row_prefix}{index}_start_date")
                } else {
                    format!("{row_prefix}{index}")
                })
                .or_default();
        }
        let index = indices
            .last()
            .copied()
            .unwrap_or_default()
            .checked_add(1)
            .ok_or_else(|| payload::invalid("base"))?;
        draft.fields.insert(
            if prefix == "step" {
                format!("{row_prefix}{index}_start_date")
            } else {
                format!("{row_prefix}{index}")
            },
            String::new(),
        );
        return Ok(());
    }
    if let Some(row) = intent.strip_prefix("remove_") {
        if !row.starts_with("time_") && !row.starts_with("date_") && !row.starts_with("step_") {
            return Err(payload::invalid("base"));
        }
        draft
            .fields
            .retain(|name, _| name != row && !name.starts_with(&format!("{row}_")));
        return Ok(());
    }
    Err(payload::invalid("base"))
}
