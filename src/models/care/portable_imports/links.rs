use super::*;

pub(super) async fn write_location_memberships(
    db: &DatabaseTransaction,
    tenant: &TenantTransaction,
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
            [tenant.scope().household_id.into(), person_id.into(), location_id.into()]))
            .await.map_err(|_| "Imported location membership could not be created".to_owned())?;
    }
    Ok(())
}

pub(super) async fn write_health_links(
    db: &DatabaseTransaction,
    tenant: &TenantTransaction,
    row: &Value,
    index: &ExistingIndex,
    event_id: i64,
) -> Result<(), String> {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "DELETE FROM health_event_medications WHERE household_id = $1 AND health_event_id = $2",
        [tenant.scope().household_id.into(), event_id.into()],
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
            [tenant.scope().household_id.into(), event_id.into(), medication_id.into()]))
            .await.map_err(|_| "Health event medication could not be restored".to_owned())?;
    }
    Ok(())
}
