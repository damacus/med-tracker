use super::*;

async fn list(
    state: AppState,
    household_id: i64,
    id: String,
    headers: HeaderMap,
    query: Result<Query<RangeQuery>, QueryRejection>,
    kind: Kind,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let Some(source) = find_source(&db, &context, kind, &id).await? else {
        return fail(db, &context, kind, "GET", "index", Failure::NotFound).await;
    };
    let Query(query) = match query {
        Ok(query) => query,
        Err(_) => {
            return fail(
                db,
                &context,
                kind,
                "GET",
                "index",
                Failure::Invalid("date_range", "is invalid"),
            )
            .await
        }
    };
    let dates = query
        .start_date
        .as_deref()
        .and_then(date)
        .zip(query.end_date.as_deref().and_then(date));
    let Some((start, end)) =
        dates.filter(|(start, end)| end >= start && (*end - *start).num_days() <= 30)
    else {
        return fail(
            db,
            &context,
            kind,
            "GET",
            "index",
            Failure::Invalid("date_range", "must contain at most 31 inclusive days"),
        )
        .await;
    };
    let secret = state.oauth.occurrence_key_secret();
    let rows = projected(&db, &source, start, end).await?;
    let data: Vec<Value> = rows
        .iter()
        .map(|row| row_value(&secret, &source, row))
        .collect();
    let request_id = Uuid::new_v4().to_string();
    finish(
        db,
        &context,
        kind,
        "GET",
        "index",
        json!({"data": data}),
        None,
        &request_id,
    )
    .await
}

pub(crate) async fn list_schedule(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    query: Result<Query<RangeQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    list(state, household_id, id, headers, query, Kind::Schedule).await
}

pub(crate) async fn list_assignment(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    query: Result<Query<RangeQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    list(state, household_id, id, headers, query, Kind::Assignment).await
}
