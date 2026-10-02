mod access;
mod browser_guard;
mod input;
mod legacy;
mod persistence;
mod reading;
mod replay;
mod representation;
mod responses;
mod sync;
mod writing;

pub(crate) use browser_guard::BrowserSourceGuard;

use crate::audit;
use crate::dosage_options::valid_identifier;
use crate::entities::{grant, membership, pause_period, person, person_medication, schedule};
use crate::medication_management::{
    error_response, finish_with_request_id, record_version, request_context,
};
use crate::mutation_idempotency::{self, Lookup, StoredResponse};
use crate::read_entities::{
    person as read_person, person_medication as read_assignment, schedule as read_schedule,
};
use crate::read_resources::{serialize_assignments, serialize_schedules};
use crate::sync_batch::{SyncOperation, SyncResult};
use crate::sync_events::{record_change, SyncRecord};
use crate::{database_error, representation_etag, ApiError, AppState, AuthContext};
use axum::body::Bytes;
use axum::extract::{rejection::JsonRejection, Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseTransaction, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

const PERIOD_CONTROLLER: &str = "api/v1/medication_pause_periods";
const PERIOD_POLICY: &str = "MedicationPausePeriodPolicy";
const REASONS: &[&str] = &[
    "out_of_supply",
    "temporarily_not_needed",
    "clinician_advice",
    "side_effects",
    "other",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Schedule,
    Assignment,
}

impl Kind {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "schedule" => Some(Self::Schedule),
            "person_medication" => Some(Self::Assignment),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Schedule => "schedule",
            Self::Assignment => "person_medication",
        }
    }

    fn policy(self) -> &'static str {
        match self {
            Self::Schedule => "SchedulePolicy",
            Self::Assignment => "PersonMedicationPolicy",
        }
    }

    fn controller(self) -> &'static str {
        match self {
            Self::Schedule => "api/v1/schedules",
            Self::Assignment => "api/v1/person_medications",
        }
    }

    fn record_type(self) -> &'static str {
        match self {
            Self::Schedule => "Schedule",
            Self::Assignment => "PersonMedication",
        }
    }
}

enum Source {
    Schedule(schedule::Model),
    Assignment(person_medication::Model),
}

impl Source {
    fn kind(&self) -> Kind {
        match self {
            Self::Schedule(_) => Kind::Schedule,
            Self::Assignment(_) => Kind::Assignment,
        }
    }

    fn id(&self) -> i64 {
        match self {
            Self::Schedule(row) => row.id,
            Self::Assignment(row) => row.id,
        }
    }

    fn person_id(&self) -> i64 {
        match self {
            Self::Schedule(row) => row.person_id,
            Self::Assignment(row) => row.person_id,
        }
    }

    fn portable_id(&self) -> &str {
        match self {
            Self::Schedule(row) => &row.portable_id,
            Self::Assignment(row) => &row.portable_id,
        }
    }

    fn active(&self) -> bool {
        match self {
            Self::Schedule(row) => row.active,
            Self::Assignment(row) => row.active,
        }
    }

    fn retired(&self) -> bool {
        match self {
            Self::Schedule(row) => row.retired_at.is_some(),
            Self::Assignment(row) => row.retired_at.is_some(),
        }
    }
}

enum Failure {
    PreconditionRequired,
    Malformed,
    Invalid(&'static str, &'static str),
    NotFound,
    Forbidden,
    Conflict,
    KeyConflict,
}

#[derive(Deserialize)]
pub(super) struct ListQuery {
    page: Option<i64>,
    per_page: Option<i64>,
    source_type: Option<String>,
    source_id: Option<String>,
}

enum Replay {
    New,
    Saved(Response),
    Conflict,
}

use access::{actor_names, find_source, find_source_path, person_access, source_for_period};
use input::{create_attributes, empty_request, source_identity};
pub(super) use legacy::{
    pause_assignment, pause_schedule, reorder_assignment, resume_assignment, resume_schedule,
};
use persistence::{close_period, open_period, pause_source};
pub(super) use reading::index;
use replay::{keyed_replay, store_key};
pub(super) use representation::period_values;
use representation::{period_body, period_row, period_snapshot, source_body};
use responses::{fail, finish, keyed_invalid};
pub(super) use sync::{apply_sync_operation, authorize_sync_replay};
pub(super) use writing::{create, resume};
