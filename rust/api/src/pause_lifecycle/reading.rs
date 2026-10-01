use super::*;

pub(crate) async fn index(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    query: Result<Query<ListQuery>, axum::extract::rejection::QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let Query(query) = match query {
        Ok(query) => query,
        Err(_) => {
            return fail(
                db,
                &context,
                "GET",
                PERIOD_CONTROLLER,
                PERIOD_POLICY,
                "index",
                Failure::Invalid("query", "is invalid"),
            )
            .await;
        }
    };
    let (page, per_page) = (query.page.unwrap_or(1), query.per_page.unwrap_or(20));
    if page < 1 || !(1..=100).contains(&per_page) {
        return fail(
            db,
            &context,
            "GET",
            PERIOD_CONTROLLER,
            PERIOD_POLICY,
            "index",
            Failure::Invalid("pagination", "is invalid"),
        )
        .await;
    }
    let source_filter = match (query.source_type.as_deref(), query.source_id.as_deref()) {
        (None, None) => None,
        (Some(kind), Some(id)) => {
            let Some(kind) = Kind::parse(kind) else {
                return fail(
                    db,
                    &context,
                    "GET",
                    PERIOD_CONTROLLER,
                    PERIOD_POLICY,
                    "index",
                    Failure::Invalid("source_type", "is invalid"),
                )
                .await;
            };
            if Uuid::parse_str(id).is_err() {
                return fail(
                    db,
                    &context,
                    "GET",
                    PERIOD_CONTROLLER,
                    PERIOD_POLICY,
                    "index",
                    Failure::Invalid("source_id", "is invalid"),
                )
                .await;
            }
            let Some(source) = find_source(&db, &context, kind, id, true).await? else {
                return fail(
                    db,
                    &context,
                    "GET",
                    PERIOD_CONTROLLER,
                    PERIOD_POLICY,
                    "index",
                    Failure::NotFound,
                )
                .await;
            };
            Some(source)
        }
        _ => {
            return fail(
                db,
                &context,
                "GET",
                PERIOD_CONTROLLER,
                PERIOD_POLICY,
                "index",
                Failure::Invalid("source", "source_type and source_id are required together"),
            )
            .await;
        }
    };
    let visible_ids: HashSet<i64> = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(grant::Column::AccessLevel.is_in(["view", "record", "manage"]))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
        )
        .all(&db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|grant| grant.person_id)
        .collect();
    let schedules = schedule::Entity::find()
        .filter(schedule::Column::HouseholdId.eq(household_id))
        .filter(schedule::Column::PersonId.is_in(visible_ids.iter().copied().collect::<Vec<_>>()))
        .all(&db)
        .await
        .map_err(database_error)?;
    let assignments = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(household_id))
        .filter(
            person_medication::Column::PersonId
                .is_in(visible_ids.iter().copied().collect::<Vec<_>>()),
        )
        .all(&db)
        .await
        .map_err(database_error)?;
    let mut sources: HashMap<(String, i64), Source> = HashMap::new();
    for row in schedules {
        sources.insert(("schedule".to_owned(), row.id), Source::Schedule(row));
    }
    for row in assignments {
        sources.insert(
            ("person_medication".to_owned(), row.id),
            Source::Assignment(row),
        );
    }
    let schedule_ids: Vec<i64> = sources
        .iter()
        .filter_map(|((kind, id), _)| (kind == "schedule").then_some(*id))
        .collect();
    let assignment_ids: Vec<i64> = sources
        .iter()
        .filter_map(|((kind, id), _)| (kind == "person_medication").then_some(*id))
        .collect();
    let mut period_query = pause_period::Entity::find()
        .filter(pause_period::Column::HouseholdId.eq(household_id))
        .filter(
            Condition::any()
                .add(pause_period::Column::ScheduleId.is_in(schedule_ids))
                .add(pause_period::Column::PersonMedicationId.is_in(assignment_ids)),
        );
    if let Some(selected) = source_filter.as_ref() {
        period_query = match selected {
            Source::Schedule(source) => {
                period_query.filter(pause_period::Column::ScheduleId.eq(source.id))
            }
            Source::Assignment(source) => {
                period_query.filter(pause_period::Column::PersonMedicationId.eq(source.id))
            }
        };
    }
    let total = period_query
        .clone()
        .count(&db)
        .await
        .map_err(database_error)?;
    let start = ((page - 1) as u64).saturating_mul(per_page as u64);
    let periods = period_query
        .order_by_desc(pause_period::Column::CreatedAt)
        .order_by_desc(pause_period::Column::Id)
        .limit(per_page as u64)
        .offset(start)
        .all(&db)
        .await
        .map_err(database_error)?;
    let names = actor_names(&db, &periods).await?;
    let mut data = Vec::new();
    for period in periods {
        let key = if let Some(id) = period.schedule_id {
            ("schedule".to_owned(), id)
        } else {
            (
                "person_medication".to_owned(),
                period.person_medication_id.unwrap(),
            )
        };
        if let Some(source) = sources.get(&key) {
            data.push(period_row(&period, source, &names));
        }
    }
    let body =
        json!({"data": data, "meta": {"page": page, "per_page": per_page, "total_count": total}});
    let request_id = Uuid::new_v4().to_string();
    finish(
        db,
        &context,
        "GET",
        PERIOD_CONTROLLER,
        PERIOD_POLICY,
        "index",
        StatusCode::OK,
        body,
        None,
        &request_id,
    )
    .await
}
