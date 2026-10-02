use super::*;

pub(crate) fn take_etag(take: &medication_take::Model) -> String {
    let input = format!(
        "MedicationTake:{}:{}",
        take.id,
        take.updated_at.and_utc().timestamp_micros()
    );
    format!("\"{}\"", hex::encode(Sha256::digest(input.as_bytes())))
}

pub(crate) async fn serialize(
    db: &DatabaseTransaction,
    takes: &[medication_take::Model],
) -> Result<Vec<Value>, ApiError> {
    let schedule_ids: Vec<i64> = takes.iter().filter_map(|take| take.schedule_id).collect();
    let assignment_ids: Vec<i64> = takes
        .iter()
        .filter_map(|take| take.person_medication_id)
        .collect();
    let inventory_ids: Vec<i64> = takes
        .iter()
        .filter_map(|take| take.taken_from_medication_id)
        .collect();
    let location_ids: Vec<i64> = takes
        .iter()
        .filter_map(|take| take.taken_from_location_id)
        .collect();
    let schedules: HashMap<_, _> = if schedule_ids.is_empty() {
        HashMap::new()
    } else {
        schedule::Entity::find()
            .filter(schedule::Column::Id.is_in(schedule_ids))
            .all(db)
            .await
            .map_err(database_error)?
            .into_iter()
            .map(|value| (value.id, value))
            .collect()
    };
    let assignments: HashMap<_, _> = if assignment_ids.is_empty() {
        HashMap::new()
    } else {
        person_medication::Entity::find()
            .filter(person_medication::Column::Id.is_in(assignment_ids))
            .all(db)
            .await
            .map_err(database_error)?
            .into_iter()
            .map(|value| (value.id, value))
            .collect()
    };
    let inventory: HashMap<_, _> = if inventory_ids.is_empty() {
        HashMap::new()
    } else {
        medication::Entity::find()
            .filter(medication::Column::Id.is_in(inventory_ids))
            .all(db)
            .await
            .map_err(database_error)?
            .into_iter()
            .map(|value| (value.id, value.portable_id))
            .collect()
    };
    let locations: HashMap<_, _> = if location_ids.is_empty() {
        HashMap::new()
    } else {
        location::Entity::find()
            .filter(location::Column::Id.is_in(location_ids))
            .all(db)
            .await
            .map_err(database_error)?
            .into_iter()
            .map(|value| (value.id, value.portable_id))
            .collect()
    };
    let med_ids: Vec<i64> = schedules
        .values()
        .map(|v| v.medication_id)
        .chain(assignments.values().map(|v| v.medication_id))
        .collect();
    let meds: HashMap<_, _> = if med_ids.is_empty() {
        HashMap::new()
    } else {
        medication::Entity::find()
            .filter(medication::Column::Id.is_in(med_ids))
            .all(db)
            .await
            .map_err(database_error)?
            .into_iter()
            .map(|value| (value.id, value.portable_id))
            .collect()
    };
    let person_ids: Vec<i64> = schedules
        .values()
        .map(|v| v.person_id)
        .chain(assignments.values().map(|v| v.person_id))
        .collect();
    let people: HashMap<_, _> = if person_ids.is_empty() {
        HashMap::new()
    } else {
        crate::entities::person::Entity::find()
            .filter(crate::entities::person::Column::Id.is_in(person_ids))
            .all(db)
            .await
            .map_err(database_error)?
            .into_iter()
            .map(|value| (value.id, value))
            .collect()
    };
    Ok(takes.iter().map(|take| {
        let schedule = take.schedule_id.and_then(|id| schedules.get(&id));
        let assignment = take.person_medication_id.and_then(|id| assignments.get(&id));
        let person_id = schedule.map(|v| v.person_id).or_else(|| assignment.map(|v| v.person_id));
        let medication_id = schedule.map(|v| v.medication_id).or_else(|| assignment.map(|v| v.medication_id));
        json!({
            "id": take.id, "portable_id": take.portable_id, "client_uuid": take.client_uuid,
            "schedule_id": take.schedule_id, "schedule_portable_id": schedule.map(|v| &v.portable_id),
            "person_medication_id": take.person_medication_id, "person_medication_portable_id": assignment.map(|v| &v.portable_id),
            "taken_from_medication_id": take.taken_from_medication_id, "taken_from_medication_portable_id": take.taken_from_medication_id.and_then(|id| inventory.get(&id)),
            "taken_from_location_id": take.taken_from_location_id, "taken_from_location_portable_id": take.taken_from_location_id.and_then(|id| locations.get(&id)),
            "dose_amount": take.dose_amount.map(|amount| decimal_string(amount.to_string())), "dose_unit": take.dose_unit,
            "taken_at": take.taken_at.map(|value| value.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
            "updated_at": take.updated_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            "person_id": person_id, "person_portable_id": person_id.and_then(|id| people.get(&id)).map(|v| &v.portable_id),
            "medication_id": medication_id, "medication_portable_id": medication_id.and_then(|id| meds.get(&id))
        })
    }).collect())
}

pub async fn index(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    pagination: Result<Query<Pagination>, QueryRejection>,
    headers: HeaderMap,
) -> Response {
    let request_id = Uuid::new_v4().to_string();
    let db = match state.db.begin().await {
        Ok(db) => db,
        Err(error) => return request_error_response(database_error(error), &request_id),
    };
    let context = match authenticate(&state, &db, &headers, household_id).await {
        Ok(context) => context,
        Err(error) => {
            if error.preserve_activity {
                let _ = db.commit().await;
            }
            return request_error_response(error, &request_id);
        }
    };
    let pagination = match pagination {
        Ok(Query(pagination)) => pagination,
        Err(_) => {
            let error = ApiError::invalid_pagination();
            let _ = audit(
                &db,
                &context,
                &request_id,
                "index",
                "GET",
                error.status,
                false,
            )
            .await;
            let _ = db.commit().await;
            return request_error_response(error, &request_id);
        }
    };
    let result = index_in_transaction(&db, &context, household_id, pagination).await;
    match result {
        Ok(body) => {
            if let Err(error) = audit(
                &db,
                &context,
                &request_id,
                "index",
                "GET",
                StatusCode::OK,
                true,
            )
            .await
            {
                return request_error_response(error, &request_id);
            }
            if let Err(error) = db.commit().await {
                return request_error_response(database_error(error), &request_id);
            }
            success_response(StatusCode::OK, body, &request_id, None)
        }
        Err(error) => {
            let _ = audit(
                &db,
                &context,
                &request_id,
                "index",
                "GET",
                error.status,
                false,
            )
            .await;
            let _ = db.commit().await;
            request_error_response(error, &request_id)
        }
    }
}

async fn index_in_transaction(
    db: &DatabaseTransaction,
    context: &AuthContext,
    household_id: i64,
    pagination: Pagination,
) -> Result<Value, ApiError> {
    if pagination.page.is_some_and(|page| page < 1)
        || pagination
            .per_page
            .is_some_and(|per_page| !(1..=100).contains(&per_page))
    {
        return Err(ApiError::invalid_pagination());
    }
    if pagination.updated_since.as_deref() == Some("") {
        return Err(ApiError::invalid_filter());
    }
    let page = pagination.page.unwrap_or(1);
    let per_page = pagination.per_page.unwrap_or(20);
    let updated_since = pagination
        .updated_since
        .filter(|v| !v.is_empty())
        .map(|v| {
            DateTime::parse_from_rfc3339(&v)
                .map(|v| v.naive_utc())
                .map_err(|_| ApiError::invalid_filter())
        })
        .transpose()?;
    let mut query = medication_take::Entity::find()
        .filter(medication_take::Column::HouseholdId.eq(household_id));
    if context.membership.role != "owner" && context.membership.role != "administrator" {
        let people = grant::Entity::find()
            .select_only()
            .column(grant::Column::PersonId)
            .filter(grant::Column::HouseholdId.eq(household_id))
            .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
            .filter(grant::Column::AccessLevel.is_in(["view", "record", "manage"]))
            .filter(grant::Column::RevokedAt.is_null())
            .filter(
                Condition::any()
                    .add(grant::Column::ExpiresAt.is_null())
                    .add(grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
            )
            .into_query();
        let schedules = schedule::Entity::find()
            .select_only()
            .column(schedule::Column::Id)
            .filter(schedule::Column::HouseholdId.eq(household_id))
            .filter(schedule::Column::PersonId.in_subquery(people.clone()))
            .into_query();
        let assignments = person_medication::Entity::find()
            .select_only()
            .column(person_medication::Column::Id)
            .filter(person_medication::Column::HouseholdId.eq(household_id))
            .filter(person_medication::Column::PersonId.in_subquery(people))
            .into_query();
        query = query.filter(
            Condition::any()
                .add(medication_take::Column::ScheduleId.in_subquery(schedules))
                .add(medication_take::Column::PersonMedicationId.in_subquery(assignments)),
        );
    }
    if let Some(updated_since) = updated_since {
        query = query.filter(medication_take::Column::UpdatedAt.gte(updated_since));
    }
    let total = query.clone().count(db).await.map_err(database_error)?;
    let rows = query
        .order_by_asc(medication_take::Column::Id)
        .limit(per_page as u64)
        .offset(((page - 1) * per_page) as u64)
        .all(db)
        .await
        .map_err(database_error)?;
    let data = serialize(db, &rows).await?;
    Ok(json!({"data": data, "meta": {"page": page, "per_page": per_page, "total_count": total}}))
}
