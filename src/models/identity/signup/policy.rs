use super::*;

pub(super) async fn open(transaction: &DatabaseTransaction) -> Result<bool, SignupError> {
    match std::env::var("INVITE_ONLY") {
        Ok(value) => {
            return Ok(matches!(
                value.as_str(),
                "" | "0" | "f" | "F" | "false" | "FALSE" | "off" | "OFF"
            ));
        }
        Err(std::env::VarError::NotUnicode(_)) => return Err(SignupError::Unavailable),
        Err(std::env::VarError::NotPresent) => {}
    }
    transaction
        .execute_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT pg_advisory_xact_lock(1920296809, 1)".to_owned(),
        ))
        .await
        .map_err(unavailable)?;
    let existing = transaction
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT invite_only FROM app_settings ORDER BY id LIMIT 1 FOR UPDATE".to_owned(),
        ))
        .await
        .map_err(unavailable)?;
    if let Some(row) = existing {
        return Ok(!row
            .try_get::<bool>("", "invite_only")
            .map_err(unavailable)?);
    }
    let row = transaction
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT public.registration_has_active_owner() AS has_owner".to_owned(),
        ))
        .await
        .map_err(unavailable)?
        .ok_or(SignupError::Unavailable)?;
    let closed: bool = row.try_get("", "has_owner").map_err(unavailable)?;
    transaction
        .execute_raw(sql(
            "INSERT INTO app_settings(invite_only,created_at,updated_at) VALUES($1,now(),now())",
            [closed.into()],
        ))
        .await
        .map_err(unavailable)?;
    Ok(!closed)
}
