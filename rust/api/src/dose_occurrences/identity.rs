use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Schedule,
    Assignment,
}

impl Kind {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Schedule => "schedule",
            Self::Assignment => "person_medication",
        }
    }

    pub(super) fn controller(self) -> &'static str {
        "api/v1/dose_occurrences"
    }

    pub(super) fn policy(self) -> &'static str {
        match self {
            Self::Schedule => "SchedulePolicy",
            Self::Assignment => "PersonMedicationPolicy",
        }
    }

    pub(super) fn path_segment(self) -> &'static str {
        match self {
            Self::Schedule => "schedules",
            Self::Assignment => "person_medications",
        }
    }
}

pub(super) enum Source {
    Schedule(schedule::Model),
    Assignment(person_medication::Model),
}

impl Source {
    pub(super) fn kind(&self) -> Kind {
        match self {
            Self::Schedule(_) => Kind::Schedule,
            Self::Assignment(_) => Kind::Assignment,
        }
    }

    pub(super) fn id(&self) -> i64 {
        match self {
            Self::Schedule(row) => row.id,
            Self::Assignment(row) => row.id,
        }
    }

    pub(super) fn person_id(&self) -> i64 {
        match self {
            Self::Schedule(row) => row.person_id,
            Self::Assignment(row) => row.person_id,
        }
    }

    pub(super) fn portable_id(&self) -> &str {
        match self {
            Self::Schedule(row) => &row.portable_id,
            Self::Assignment(row) => &row.portable_id,
        }
    }

    pub(super) fn active(&self) -> bool {
        match self {
            Self::Schedule(row) => row.active,
            Self::Assignment(row) => row.active,
        }
    }

    pub(super) fn created_at(&self) -> NaiveDateTime {
        match self {
            Self::Schedule(row) => row.created_at,
            Self::Assignment(row) => row.created_at,
        }
    }

    pub(super) fn max_daily_doses(&self) -> i32 {
        match self {
            Self::Schedule(row) => row.max_daily_doses.unwrap_or(1),
            Self::Assignment(row) => row.max_daily_doses.unwrap_or(1),
        }
    }

    pub(super) fn dose_cycle(&self) -> i32 {
        match self {
            Self::Schedule(row) => row.dose_cycle.unwrap_or(0),
            Self::Assignment(row) => row.dose_cycle.unwrap_or(0),
        }
    }
}

pub(super) async fn person_access(
    db: &DatabaseTransaction,
    context: &AuthContext,
    person_id: i64,
    action: &str,
) -> Result<bool, ApiError> {
    let levels: &[&str] = match action {
        "reopen" => &["manage"],
        "not_taken" | "take" => &["record", "manage"],
        _ => &["view", "record", "manage"],
    };
    Ok(grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(context.membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(grant::Column::PersonId.eq(person_id))
        .filter(grant::Column::AccessLevel.is_in(levels.to_vec()))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
        )
        .one(db)
        .await
        .map_err(database_error)?
        .is_some())
}

pub(super) async fn find_source(
    db: &DatabaseTransaction,
    context: &AuthContext,
    kind: Kind,
    id: &str,
) -> Result<Option<Source>, ApiError> {
    if !valid_identifier(id) {
        return Ok(None);
    }
    let household_id = context.membership.household_id;
    let source = match kind {
        Kind::Schedule => {
            let query = schedule::Entity::find()
                .filter(schedule::Column::HouseholdId.eq(household_id))
                .filter(schedule::Column::RetiredAt.is_null());
            let query = match id.parse::<i64>() {
                Ok(id) => query.filter(schedule::Column::Id.eq(id)),
                Err(_) => query.filter(schedule::Column::PortableId.eq(id)),
            };
            query
                .one(db)
                .await
                .map_err(database_error)?
                .map(Source::Schedule)
        }
        Kind::Assignment => {
            let query = person_medication::Entity::find()
                .filter(person_medication::Column::HouseholdId.eq(household_id))
                .filter(person_medication::Column::RetiredAt.is_null());
            let query = match id.parse::<i64>() {
                Ok(id) => query.filter(person_medication::Column::Id.eq(id)),
                Err(_) => query.filter(person_medication::Column::PortableId.eq(id)),
            };
            query
                .one(db)
                .await
                .map_err(database_error)?
                .map(Source::Assignment)
        }
    };
    match source {
        Some(source) if person_access(db, context, source.person_id(), "index").await? => {
            Ok(Some(source))
        }
        _ => Ok(None),
    }
}
