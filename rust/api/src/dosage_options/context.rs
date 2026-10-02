use super::*;

pub(super) async fn write_context(
    state: &AppState,
    headers: &HeaderMap,
    household_id: i64,
    method: &str,
    action: &str,
) -> Result<Result<(DatabaseTransaction, AuthContext), Response>, ApiError> {
    let (db, context) = request_context(state, headers, household_id).await?;
    if household_manager(&context) {
        Ok(Ok((db, context)))
    } else {
        Ok(Err(failure(
            db,
            &context,
            method,
            action,
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
        )
        .await?))
    }
}

pub(super) async fn medication_row(
    db: &DatabaseTransaction,
    household_id: i64,
    id: &str,
) -> Result<Option<medication::Model>, ApiError> {
    let query = medication::Entity::find().filter(medication::Column::HouseholdId.eq(household_id));
    let query = if let Ok(id) = id.parse::<i64>() {
        query.filter(medication::Column::Id.eq(id))
    } else {
        query.filter(medication::Column::PortableId.eq(id))
    };
    query.one(db).await.map_err(database_error)
}

pub(super) async fn dosage_row(
    db: &DatabaseTransaction,
    household_id: i64,
    id: &str,
    lock: bool,
) -> Result<Option<dosage::Model>, ApiError> {
    let query = dosage::Entity::find().filter(dosage::Column::HouseholdId.eq(household_id));
    let query = if let Ok(id) = id.parse::<i64>() {
        query.filter(dosage::Column::Id.eq(id))
    } else {
        query.filter(dosage::Column::PortableId.eq(id))
    };
    let query = if lock { query.lock_exclusive() } else { query };
    query.one(db).await.map_err(database_error)
}
