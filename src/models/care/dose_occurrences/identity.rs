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
