use super::*;

pub(crate) async fn index(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    if !household_manager(&context) {
        return denied(db, &context, "GET", "index").await;
    }
    let rows = household_invitation::Entity::find()
        .filter(household_invitation::Column::HouseholdId.eq(household_id))
        .order_by_desc(household_invitation::Column::CreatedAt)
        .limit(100)
        .all(&db)
        .await
        .map_err(database_error)?;
    let now = Utc::now().naive_utc();
    let data = rows.iter().map(|row| summary(row, now)).collect::<Vec<_>>();
    finish(
        db,
        &context,
        "GET",
        CONTROLLER,
        POLICY,
        "index",
        StatusCode::OK,
        true,
        json!({"data":data}),
        None,
    )
    .await
}
