use crate::entities::{dosage, medication};
use crate::medication_management::{
    error_response, finish, finish_with_request_id, household_manager, lock_medication,
    medication_snapshot, record_version, request_context,
};
use crate::sync_events::{lock_household, record_change, SyncRecord};
use crate::{database_error, decimal_string, representation_etag, ApiError, AppState, AuthContext};
use axum::extract::{rejection::JsonRejection, rejection::QueryRejection, Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::{DateTime, Utc};
use sea_orm::prelude::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, QueryTrait, Set, TransactionTrait,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

mod context;
mod create;
mod inventory;
mod persistence;
mod reads;
mod responses;
mod sync;
mod update;
mod validation;

pub(super) use create::create;
pub(super) use reads::{index, show};
pub(super) use responses::{dosage_value, representation};
pub(super) use sync::{apply_sync_operation, authorize_sync_replay};
pub(super) use update::{patch, put};
pub(super) use validation::{parse_decimal, storage_decimal, valid_identifier};

use context::{dosage_row, medication_row, write_context};
use inventory::synchronize_inventory;
use persistence::{dosage_snapshot, record_sync};
use responses::{failure, validation};
use validation::{attributes, valid_persisted_dosage};
