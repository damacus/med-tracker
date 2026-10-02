use super::*;

pub(super) async fn grant_importer_access(
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

pub(super) async fn write_location_memberships(
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

pub(super) async fn write_health_links(
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
