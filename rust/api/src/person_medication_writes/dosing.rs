use super::*;

pub(super) fn option_matches(option: &dosage::Model, amount: Decimal, unit: &str) -> bool {
    option.amount == amount && option.unit == unit
}

pub(super) async fn resolved_dose(
    db: &DatabaseTransaction,
    context: &AuthContext,
    attrs: &Attributes,
    person: &person::Model,
    medication: &medication::Model,
    previous: Option<&person_medication::Model>,
    selected_option: Option<&dosage::Model>,
) -> Result<Option<(Decimal, String, Option<i64>)>, ApiError> {
    let options = dosage::Entity::find()
        .filter(dosage::Column::HouseholdId.eq(context.membership.household_id))
        .filter(dosage::Column::MedicationId.eq(medication.id))
        .order_by_asc(dosage::Column::Amount)
        .order_by_asc(dosage::Column::Id)
        .all(db)
        .await
        .map_err(database_error)?;
    let explicit_option = selected_option;
    if explicit_option.is_some_and(|option| option.medication_id != medication.id) {
        return Ok(None);
    }
    let amount = attrs
        .dose_amount
        .or_else(|| previous.and_then(|record| record.dose_amount));
    let unit = attrs
        .dose_unit
        .clone()
        .or_else(|| previous.and_then(|record| record.dose_unit.clone()));
    let default_option = if explicit_option.is_none() && (amount.is_none() || unit.is_none()) {
        if person.person_type != 0 {
            options.iter().find(|option| option.default_for_children)
        } else {
            None
        }
        .or_else(|| options.iter().find(|option| option.default_for_adults))
        .or_else(|| options.first())
    } else {
        None
    };
    let chosen = explicit_option.or(default_option);
    let amount = amount
        .or_else(|| chosen.map(|option| option.amount))
        .or_else(|| {
            medication
                .dose_amount
                .and_then(|amount| Decimal::from_str_exact(&amount.to_string()).ok())
        });
    let unit = unit
        .or_else(|| chosen.map(|option| option.unit.clone()))
        .or_else(|| medication.dose_unit.clone());
    let (Some(amount), Some(unit)) = (amount, unit) else {
        return Ok(None);
    };
    if !numeric_10_2(amount) || !DOSE_UNITS.contains(&unit.as_str()) {
        return Ok(None);
    }
    if explicit_option.is_some_and(|option| !option_matches(option, amount, &unit)) {
        return Ok(None);
    }
    let matching: Vec<_> = options
        .iter()
        .filter(|option| option_matches(option, amount, &unit))
        .collect();
    let option_id = if let Some(option) = explicit_option {
        Some(option.id)
    } else if matching.len() == 1 {
        Some(matching[0].id)
    } else {
        chosen
            .filter(|option| option_matches(option, amount, &unit))
            .map(|option| option.id)
    };
    Ok(Some((amount, unit, option_id)))
}
