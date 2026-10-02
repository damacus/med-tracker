use super::*;

pub(super) async fn person_access(
    db: &DatabaseTransaction,
    context: &AuthContext,
    person_id: i64,
    manage: bool,
) -> Result<bool, ApiError> {
    let mut query = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(context.membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(grant::Column::PersonId.eq(person_id))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
        );
    query = if manage {
        query.filter(grant::Column::AccessLevel.eq("manage"))
    } else {
        query.filter(grant::Column::AccessLevel.is_in(["view", "record", "manage"]))
    };
    Ok(query.one(db).await.map_err(database_error)?.is_some())
}

pub(super) async fn find_source(
    db: &DatabaseTransaction,
    context: &AuthContext,
    kind: Kind,
    id: &str,
    include_retired: bool,
) -> Result<Option<Source>, ApiError> {
    let household = context.membership.household_id;
    let found = match kind {
        Kind::Schedule => {
            let mut query = schedule::Entity::find()
                .filter(schedule::Column::HouseholdId.eq(household))
                .filter(schedule::Column::PortableId.eq(id));
            if !include_retired {
                query = query.filter(schedule::Column::RetiredAt.is_null());
            }
            query
                .one(db)
                .await
                .map_err(database_error)?
                .map(Source::Schedule)
        }
        Kind::Assignment => {
            let mut query = person_medication::Entity::find()
                .filter(person_medication::Column::HouseholdId.eq(household))
                .filter(person_medication::Column::PortableId.eq(id));
            if !include_retired {
                query = query.filter(person_medication::Column::RetiredAt.is_null());
            }
            query
                .one(db)
                .await
                .map_err(database_error)?
                .map(Source::Assignment)
        }
    };
    match found {
        Some(source) if person_access(db, context, source.person_id(), false).await? => {
            Ok(Some(source))
        }
        _ => Ok(None),
    }
}

pub(super) async fn find_source_path(
    db: &DatabaseTransaction,
    context: &AuthContext,
    kind: Kind,
    id: &str,
) -> Result<Option<Source>, ApiError> {
    let portable = if valid_identifier(id) {
        if let Ok(numeric) = id.parse::<i64>() {
            match kind {
                Kind::Schedule => schedule::Entity::find_by_id(numeric)
                    .one(db)
                    .await
                    .map_err(database_error)?
                    .map(|row| row.portable_id),
                Kind::Assignment => person_medication::Entity::find_by_id(numeric)
                    .one(db)
                    .await
                    .map_err(database_error)?
                    .map(|row| row.portable_id),
            }
        } else {
            Some(id.to_owned())
        }
    } else {
        None
    };
    let Some(portable) = portable else {
        return Ok(None);
    };
    find_source(db, context, kind, &portable, false).await
}

pub(super) async fn source_for_period(
    db: &DatabaseTransaction,
    context: &AuthContext,
    period: &pause_period::Model,
    include_retired: bool,
) -> Result<Option<Source>, ApiError> {
    let source = if let Some(id) = period.schedule_id {
        schedule::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .map(Source::Schedule)
    } else if let Some(id) = period.person_medication_id {
        person_medication::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .map(Source::Assignment)
    } else {
        None
    };
    match source {
        Some(source)
            if source.person_id() > 0
                && (include_retired || !source.retired())
                && person_access(db, context, source.person_id(), false).await? =>
        {
            Ok(Some(source))
        }
        _ => Ok(None),
    }
}

pub(super) async fn actor_names(
    db: &DatabaseTransaction,
    periods: &[pause_period::Model],
) -> Result<HashMap<i64, String>, ApiError> {
    let ids: Vec<i64> = periods
        .iter()
        .flat_map(|period| {
            [
                period.recorded_by_membership_id,
                period.resumed_by_membership_id,
            ]
        })
        .flatten()
        .collect();
    let actors = if ids.is_empty() {
        Vec::new()
    } else {
        membership::Entity::find()
            .filter(membership::Column::Id.is_in(ids))
            .all(db)
            .await
            .map_err(database_error)?
    };
    let people_ids: Vec<i64> = actors.iter().filter_map(|actor| actor.person_id).collect();
    let people: HashMap<i64, String> = if people_ids.is_empty() {
        HashMap::new()
    } else {
        read_person::Entity::find()
            .filter(read_person::Column::Id.is_in(people_ids))
            .all(db)
            .await
            .map_err(database_error)?
            .into_iter()
            .map(|row| (row.id, row.name))
            .collect()
    };
    Ok(actors
        .into_iter()
        .filter_map(|actor| {
            actor
                .person_id
                .and_then(|id| people.get(&id).cloned())
                .map(|name| (actor.id, name))
        })
        .collect())
}
