use super::*;

pub(super) async fn insert_take(
    db: &DatabaseTransaction,
    household_id: HouseholdId,
    proposed: &ValidatedTake,
    client_uuid: Option<Uuid>,
) -> Result<Option<medication_take::Model>, ApiError> {
    let now = Utc::now().naive_utc();
    let model = medication_take::ActiveModel {
        household_id: Set(household_id.get()),
        client_uuid: Set(client_uuid.map(|id| id.hyphenated().to_string())),
        schedule_id: Set(proposed.source.identity.schedule_id()),
        person_medication_id: Set(proposed.source.identity.assignment_id()),
        taken_from_medication_id: Set(Some(proposed.selected.id.get())),
        taken_from_location_id: Set(proposed
            .selected
            .location_id
            .map(crate::models::ids::LocationId::get)),
        dose_amount: Set(Some(proposed.amount)),
        dose_unit: Set(Some(proposed.unit.clone())),
        taken_at: Set(Some(proposed.taken_at)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    if client_uuid.is_none() {
        return model.insert(db).await.map(Some).map_err(database_error);
    }
    let statement = medication_take::Entity::insert(model)
        .on_conflict(
            sea_orm::sea_query::OnConflict::new()
                .expr(sea_orm::sea_query::Expr::cust(
                    migration::CANONICAL_CLIENT_UUID_SQL,
                ))
                .do_nothing()
                .to_owned(),
        )
        .exec_with_returning(db)
        .await;
    match statement {
        Ok(row) => Ok(Some(row)),
        Err(sea_orm::DbErr::RecordNotInserted | sea_orm::DbErr::RecordNotFound(_)) => Ok(None),
        Err(error) => Err(database_error(error)),
    }
}
