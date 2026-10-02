mod acceptance;
mod acceptance_effects;
mod input;
mod issuing;
mod mail;
mod reading;
mod replay;
mod resending;
mod responses;
mod tokens;

use crate::entities::{
    api_change_event, grant, household, household_invitation, household_invitation_grant,
    membership, security_audit_event, version,
};
use crate::medication_management::{
    error_response, finish, finish_with_request_id, household_manager, record_version,
    request_context,
};
use crate::mutation_idempotency::{self, Lookup, StoredResponse};
use crate::read_entities::{carer_relationship, person};
use crate::{
    audit, auth_sessions, database_error, restricted_role, tenant_setting, ApiError, AppState,
    AuthContext, CredentialKind,
};
use axum::body::{Body, Bytes};
use axum::extract::{rejection::JsonRejection, Path, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{Duration, NaiveDateTime, Utc};
use lettre::message::{header::ContentType, Mailbox};
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use lettre::{Message, SmtpTransport, Transport};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, DbBackend, EntityTrait,
    IntoActiveModel, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, Statement,
    TransactionTrait,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::time::Duration as StdDuration;
use url::Url;
use uuid::Uuid;

pub(super) use acceptance::accept;
use acceptance_effects::record_acceptance_effects;
use input::accepted_body;
use input::collection_path;
use input::parse_create;
use input::state_row;
use input::summary;
use input::valid_email;
use input::valid_id;
pub(super) use issuing::create;
pub(super) use issuing::destroy;
use mail::smtp_send;
pub(super) use mail::MailConfig;
pub(super) use reading::index;
use replay::accepted_retry;
use replay::keyed_failure;
use replay::keyed_replay;
pub(super) use resending::resend;
use responses::acceptance_error;
use responses::audit_event;
use responses::denied;
use responses::invalid;
use responses::invitation_unavailable;
use responses::no_store;
use responses::resend_unavailable;
use responses::CONTROLLER;
use responses::POLICY;
use tokens::new_token;
