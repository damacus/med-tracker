mod calendar;
mod identity;
mod input;
mod keys;
mod persistence;
mod projection;
mod reading;
mod replay;
mod representation;
mod responses;
mod scheduling;
mod sync;
mod writing;

use crate::audit;
use crate::dosage_options::{parse_decimal, valid_identifier};
use crate::dose;
use crate::entities::{
    dose_occurrence, grant, medication_take, pause_period, person, person_medication, schedule,
};
use crate::medication_management::{
    error_response, finish_with_request_id, record_version, request_context,
};
use crate::mutation_idempotency::{self, Lookup, StoredResponse};
use crate::sync_batch::{SyncOperation, SyncResult};
use crate::sync_events::{record_change, SyncRecord};
use crate::{database_error, representation_etag, ApiError, AppState, AuthContext};
use axum::extract::{
    rejection::{JsonRejection, QueryRejection},
    Path, Query, State,
};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc};
use hmac::{Hmac, Mac};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseTransaction, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, QueryTrait, Set,
};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use sha2::Sha256;
use std::collections::BTreeMap;
use std::sync::Arc;
use uuid::Uuid;

use calendar::config_times;
use calendar::cycle_bounds;
use calendar::date;
use calendar::local_midnight;
#[cfg(test)]
use calendar::scheduled_time_in_zone;
pub(crate) use calendar::with_dashboard_timezone;
use identity::find_source;
use identity::person_access;
use identity::Kind;
use identity::Source;
use input::attributes;
use input::parse_not_taken;
use input::parse_take;
use keys::decode_key;
use keys::key;
use persistence::actionable;
use persistence::find_row;
use persistence::link_take;
use persistence::reopen_decision;
use persistence::save_decision;
use projection::projected;
use projection::Occurrence;
pub(super) use reading::list_assignment;
pub(super) use reading::list_schedule;
use replay::keyed_replay;
use replay::store_key;
use replay::Replay;
pub(super) use representation::record_etag;
use representation::row_value;
use representation::snapshot;
use responses::fail;
use responses::fail_api;
use responses::fail_mutation;
use responses::finish;
use responses::finish_cached_error;
use responses::sync_error;
use responses::Failure;
use scheduling::effective_count;
use scheduling::schedule_applies;
use scheduling::schedule_as_needed;
use scheduling::schedule_config_on;
pub(super) use sync::apply_sync_operation;
pub(super) use sync::authorize_sync_replay;
pub(super) use writing::not_taken_assignment;
pub(super) use writing::not_taken_schedule;
pub(super) use writing::reopen_assignment;
pub(super) use writing::reopen_schedule;
pub(super) use writing::take_assignment;
pub(super) use writing::take_schedule;

#[derive(Deserialize)]
pub(super) struct RangeQuery {
    start_date: Option<String>,
    end_date: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduled_time_preserves_minutes_across_daylight_saving_gap() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 29).unwrap();
        let actual = scheduled_time_in_zone(date, "01:30", chrono_tz::Europe::London).unwrap();
        let expected = date.and_hms_opt(1, 30, 0).unwrap();
        assert_eq!(actual, expected);
    }
}
