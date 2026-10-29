use super::*;

async fn lock<E>(db: &DatabaseTransaction, id: i64) -> Result<(), ApiError>
where
    E: EntityTrait,
    E::PrimaryKey: PrimaryKeyTrait<ValueType = i64>,
{
    E::find_by_id(id)
        .lock_exclusive()
        .one(db)
        .await
        .map_err(OperationError::from)?
        .ok_or(OperationError::NotFound)?;
    Ok(())
}

pub(super) async fn lock_household(db: &DatabaseTransaction, id: i64) -> Result<(), ApiError> {
    lock::<household::Entity>(db, id).await
}

pub(super) async fn lock_medication(db: &DatabaseTransaction, id: i64) -> Result<(), ApiError> {
    lock::<medication::Entity>(db, id).await
}

pub(super) async fn lock_dosage(db: &DatabaseTransaction, id: i64) -> Result<(), ApiError> {
    lock::<dosage::Entity>(db, id).await
}
