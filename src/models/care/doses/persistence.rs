use super::*;
use sea_orm::sea_query::ExprTrait;

pub(super) async fn insert_take(
    db: &DatabaseTransaction,
    household_id: i64,
    proposed: &ProposedTake,
    client_uuid: Option<&str>,
) -> Result<Option<medication_take::Model>, ApiError> {
    let now = Utc::now().naive_utc();
    let model = medication_take::ActiveModel {
        household_id: Set(household_id),
        client_uuid: Set(client_uuid.map(str::to_owned)),
        schedule_id: Set((proposed.source.kind == "schedule").then_some(proposed.source.id)),
        person_medication_id: Set(
            (proposed.source.kind == "person_medication").then_some(proposed.source.id)
        ),
        taken_from_medication_id: Set(Some(proposed.selected.id)),
        taken_from_location_id: Set(Some(proposed.selected.location_id)),
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
            sea_orm::sea_query::OnConflict::column(medication_take::Column::ClientUuid)
                .target_and_where(
                    sea_orm::sea_query::Expr::col(medication_take::Column::ClientUuid)
                        .is_not_null(),
                )
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
