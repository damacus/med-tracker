use super::*;

#[derive(Deserialize)]
pub(crate) struct Pagination {
    page: Option<i64>,
    per_page: Option<i64>,
    updated_since: Option<String>,
}

pub(crate) async fn index(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    pagination: Result<Query<Pagination>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let Query(pagination) = match pagination {
        Ok(value) => value,
        Err(_) => return validation(db, &context, "GET", "index").await,
    };
    let page = pagination.page.unwrap_or(1);
    let per_page = pagination.per_page.unwrap_or(20);
    if page < 1 || !(1..=100).contains(&per_page) {
        return validation(db, &context, "GET", "index").await;
    }
    let updated_since = match pagination.updated_since {
        Some(value) => match DateTime::parse_from_rfc3339(&value) {
            Ok(value) => Some(value.naive_utc()),
            Err(_) => return validation(db, &context, "GET", "index").await,
        },
        None => None,
    };
    let visible_medication_ids = crate::scope(household_id, &context.membership)
        .select_only()
        .column(medication::Column::Id)
        .into_query();
    let mut query = dosage::Entity::find()
        .filter(dosage::Column::HouseholdId.eq(household_id))
        .filter(dosage::Column::MedicationId.in_subquery(visible_medication_ids));
    if let Some(updated_since) = updated_since {
        query = query.filter(dosage::Column::UpdatedAt.gte(updated_since));
    }
    let total_count = query.clone().count(&db).await.map_err(database_error)?;
    let records = query
        .order_by_asc(dosage::Column::Id)
        .limit(per_page as u64)
        .offset(page.saturating_sub(1).saturating_mul(per_page) as u64)
        .all(&db)
        .await
        .map_err(database_error)?;
    let medication_ids: Vec<i64> = records.iter().map(|record| record.medication_id).collect();
    let medication_ids: HashMap<i64, String> = medication::Entity::find()
        .filter(medication::Column::Id.is_in(medication_ids))
        .all(&db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|record| (record.id, record.portable_id))
        .collect();
    let rows: Vec<Value> = records
        .into_iter()
        .map(|record| {
            let portable = medication_ids
                .get(&record.medication_id)
                .ok_or_else(ApiError::internal)?;
            dosage_value(record, portable)
        })
        .collect::<Result<_, _>>()?;
    finish(db, &context, "GET", "api/v1/dosage_options", "DosageOptionPolicy", "index", StatusCode::OK, true, json!({"data": rows, "meta": {"page": page, "per_page": per_page, "total_count": total_count}}), None).await
}

pub(crate) async fn show(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    if !valid_identifier(&id) {
        return failure(
            db,
            &context,
            "GET",
            "show",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    }
    let Some(record) = dosage_row(&db, household_id, &id, false).await? else {
        return failure(
            db,
            &context,
            "GET",
            "show",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    };
    let visible = crate::scope(household_id, &context.membership)
        .filter(medication::Column::Id.eq(record.medication_id))
        .one(&db)
        .await
        .map_err(database_error)?;
    if visible.is_none() {
        return failure(
            db,
            &context,
            "GET",
            "show",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    }
    let (body, etag) = representation(&db, record).await?;
    finish(
        db,
        &context,
        "GET",
        "api/v1/dosage_options",
        "DosageOptionPolicy",
        "show",
        StatusCode::OK,
        true,
        body,
        Some(&etag),
    )
    .await
}
