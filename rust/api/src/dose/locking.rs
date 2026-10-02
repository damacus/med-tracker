use super::*;

pub(super) async fn lock_row(
    db: &DatabaseTransaction,
    table: &str,
    id: i64,
) -> Result<(), ApiError> {
    let sql = match table {
        "households" => "SELECT id FROM households WHERE id = $1 FOR UPDATE",
        "medications" => "SELECT id FROM medications WHERE id = $1 FOR UPDATE",
        "dosages" => "SELECT id FROM dosages WHERE id = $1 FOR UPDATE",
        _ => return Err(ApiError::internal()),
    };
    db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        [id.into()],
    ))
    .await
    .map_err(database_error)?
    .ok_or_else(ApiError::not_found)?;
    Ok(())
}
