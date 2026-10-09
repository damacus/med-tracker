mod administration;
mod assignments;
mod audit;
mod dosages;
mod dose_history;
mod dose_occurrences;
mod input;
mod invitations;
mod locations;
mod medication_crud;
mod medication_reads;
mod notification_preferences;
mod orders;
mod pause_periods;
mod people;
mod profile;
mod projection;
mod push_subscriptions;
mod removals;
mod reports;
mod response;
mod schedule_lifecycle;
mod sync;
mod treatments;

use crate::models::{
    access::TenantTransaction,
    care::{
        doses::{self, CredentialMethod, CredentialProvenance},
        medications,
    },
    entities::{medication_take, security_audit_event},
    errors::OperationError,
    identity::resource::{self, AuthenticationError, ValidatedPrincipal},
};
use axum::{
    Extension, Json as AxumJson,
    extract::rejection::JsonRejection,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use chrono::Utc;
use loco_rs::controller::middleware::request_id::LocoRequestId;
use loco_rs::prelude::*;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub fn invitation_routes() -> Routes {
    Routes::new()
        .prefix("/api/v1")
        .add("/invitations/accept", post(invitations::accept))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/households")
        .add(
            "/{household_id}/profile",
            get(profile::show)
                .patch(profile::update)
                .put(profile::update),
        )
        .add(
            "/{household_id}/notification_preference",
            get(notification_preferences::show)
                .patch(notification_preferences::update)
                .put(notification_preferences::update),
        )
        .add("/{household_id}/sync/snapshot", get(sync::snapshot))
        .add("/{household_id}/sync/changes", get(sync::changes))
        .add("/{household_id}/sync/batches", post(sync::create))
        .add(
            "/{household_id}/reports/health_history",
            get(reports::health_json),
        )
        .add(
            "/{household_id}/reports/health_history.pdf",
            get(reports::health_pdf),
        )
        .add(
            "/{household_id}/reports/medication_reviews",
            get(reports::reviews_json),
        )
        .add(
            "/{household_id}/reports/medication_reviews.pdf",
            get(reports::reviews_pdf),
        )
        .add(
            "/{household_id}/schedules/{id}/dose_occurrences",
            get(dose_occurrences::list_schedule),
        )
        .add(
            "/{household_id}/schedules/{id}/dose_occurrences/not_taken",
            post(dose_occurrences::not_taken_schedule),
        )
        .add(
            "/{household_id}/schedules/{id}/dose_occurrences/reopen",
            patch(dose_occurrences::reopen_schedule),
        )
        .add(
            "/{household_id}/schedules/{id}/dose_occurrences/take",
            post(dose_occurrences::take_schedule),
        )
        .add(
            "/{household_id}/person_medications/{id}/dose_occurrences",
            get(dose_occurrences::list_assignment),
        )
        .add(
            "/{household_id}/person_medications/{id}/dose_occurrences/not_taken",
            post(dose_occurrences::not_taken_assignment),
        )
        .add(
            "/{household_id}/person_medications/{id}/dose_occurrences/reopen",
            patch(dose_occurrences::reopen_assignment),
        )
        .add(
            "/{household_id}/person_medications/{id}/dose_occurrences/take",
            post(dose_occurrences::take_assignment),
        )
        .add(
            "/{household_id}/schedules/{id}/pause",
            patch(pause_periods::legacy::pause_schedule),
        )
        .add(
            "/{household_id}/schedules/{id}/resume",
            patch(pause_periods::legacy::resume_schedule),
        )
        .add(
            "/{household_id}/person_medications/{id}/pause",
            patch(pause_periods::legacy::pause_assignment),
        )
        .add(
            "/{household_id}/person_medications/{id}/resume",
            patch(pause_periods::legacy::resume_assignment),
        )
        .add(
            "/{household_id}/medication_pause_periods",
            get(pause_periods::index).post(pause_periods::create),
        )
        .add(
            "/{household_id}/medication_pause_periods/{id}/resume",
            post(pause_periods::resume),
        )
        .add(
            "/{household_id}/person_medications",
            get(assignments::index).post(assignments::create),
        )
        .add(
            "/{household_id}/person_medications/{id}",
            get(assignments::show)
                .patch(assignments::update)
                .put(assignments::update),
        )
        .add(
            "/{household_id}/push_subscription",
            post(push_subscriptions::create).delete(push_subscriptions::destroy),
        )
        .add(
            "/{household_id}/schedules",
            get(schedule_lifecycle::index).post(treatments::create),
        )
        .add(
            "/{household_id}/schedules/{id}",
            get(schedule_lifecycle::show)
                .patch(schedule_lifecycle::update)
                .put(schedule_lifecycle::update),
        )
        .add(
            "/{household_id}/dosage_options",
            get(dosages::index).post(dosages::create),
        )
        .add(
            "/{household_id}/dosage_options/{id}",
            get(dosages::show).patch(dosages::patch).put(dosages::put),
        )
        .add(
            "/{household_id}/admin/invitations",
            get(invitations::index).post(invitations::create),
        )
        .add(
            "/{household_id}/admin/invitations/{id}",
            delete(invitations::destroy),
        )
        .add(
            "/{household_id}/admin/invitations/{id}/resend",
            post(invitations::resend),
        )
        .add(
            "/{household_id}/admin/settings",
            get(administration::settings_show)
                .patch(administration::settings_update)
                .put(administration::settings_update),
        )
        .add(
            "/{household_id}/admin/memberships",
            get(administration::memberships_index),
        )
        .add(
            "/{household_id}/admin/memberships/{id}",
            patch(administration::memberships_update)
                .put(administration::memberships_update)
                .delete(administration::memberships_destroy),
        )
        .add(
            "/{household_id}/admin/person_access_grants",
            get(administration::grants_index).post(administration::grants_create),
        )
        .add(
            "/{household_id}/admin/person_access_grants/{id}",
            delete(administration::grants_destroy),
        )
        .add(
            "/{household_id}/people",
            get(people::index).post(people::create),
        )
        .add(
            "/{household_id}/people/{id}",
            get(people::show).patch(people::update).put(people::update),
        )
        .add(
            "/{household_id}/locations",
            get(locations::index).post(locations::create),
        )
        .add(
            "/{household_id}/locations/{id}",
            get(locations::show)
                .patch(locations::update)
                .put(locations::update)
                .delete(locations::destroy),
        )
        .add(
            "/{household_id}/medications",
            get(medication_reads::index).post(medication_crud::create),
        )
        .add(
            "/{household_id}/medications/{id}",
            get(medication_reads::show)
                .patch(medication_crud::update)
                .put(medication_crud::update),
        )
        .add(
            "/{household_id}/medication_takes",
            get(dose_history::index).post(take),
        )
        .add(
            "/{household_id}/medications/{id}/mark_as_ordered",
            patch(orders::ordered),
        )
        .add(
            "/{household_id}/medications/{id}/mark_as_received",
            patch(orders::received),
        )
        .add(
            "/{household_id}/medications/{id}/adjust_inventory",
            patch(adjust_stock),
        )
        .add(
            "/{household_id}/medications/{id}/stock_removals",
            get(removals::history).post(removals::create),
        )
}

async fn begin(
    ctx: &AppContext,
    headers: &HeaderMap,
    household_id: i64,
    request_id: &str,
) -> std::result::Result<(ValidatedPrincipal, TenantTransaction), response::Failure> {
    let principal = resource::authenticate_care(ctx, headers)
        .await
        .map_err(response::authentication)?;
    let tenant = principal
        .begin_household(&ctx.db, household_id, request_id.into())
        .await
        .map_err(response::authentication)?;
    Ok((principal, tenant))
}

async fn begin_profile_write(
    ctx: &AppContext,
    headers: &HeaderMap,
    household_id: i64,
    request_id: &str,
) -> std::result::Result<(ValidatedPrincipal, TenantTransaction), response::Failure> {
    let principal = resource::authenticate_care(ctx, headers)
        .await
        .map_err(response::authentication)?;
    let tenant = principal
        .begin_household_for_profile_write(&ctx.db, household_id, request_id.into())
        .await
        .map_err(response::authentication)?;
    Ok((principal, tenant))
}

async fn take(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(context) => context,
        Err(error) => return response::error(error, &request_id),
    };
    let input = match body {
        Ok(AxumJson(body)) => input::take(body),
        Err(_) => Err(response::Failure::bad_request("Invalid JSON request body")),
    };
    let savepoint = match tenant.transaction().begin().await {
        Ok(savepoint) => savepoint,
        Err(_) => return response::error(response::unavailable(), &request_id),
    };
    let result = match input {
        Ok(command) => doses::execute_in_timezone(
            &tenant,
            doses::Command::Take(command),
            principal.time_zone(),
            Some(principal.provenance()),
        )
        .await
        .map_err(response::operation),
        Err(error) => Err(error),
    };
    let result = match result {
        Ok(outcome) => {
            let (status, record) = match outcome {
                doses::Outcome::Created(record) => (StatusCode::CREATED, record),
                doses::Outcome::Replayed(record) => (StatusCode::OK, record),
            };
            projection::serialize(
                tenant.transaction(),
                std::slice::from_ref(&record),
                household_id,
            )
            .await
            .map(|rows| {
                (
                    status,
                    json!({"data": rows.into_iter().next().expect("Single take projection")}),
                    Some(projection::take_etag(&record)),
                )
            })
            .map_err(response::operation)
        }
        Err(error) => Err(error),
    };
    let savepoint_result = if result.is_ok() {
        savepoint.commit().await
    } else {
        savepoint.rollback().await
    };
    if savepoint_result.is_err() {
        return response::error(response::unavailable(), &request_id);
    }
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::take(),
        result,
        &request_id,
    )
    .await
}

async fn adjust_stock(
    State(ctx): State<AppContext>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(context) => context,
        Err(error) => return response::error(error, &request_id),
    };
    let input = match body {
        Ok(AxumJson(body)) => input::stock(body, id),
        Err(_) => Err(response::Failure::bad_request("Invalid request body")),
    };
    let savepoint = match tenant.transaction().begin().await {
        Ok(savepoint) => savepoint,
        Err(_) => return response::error(response::unavailable(), &request_id),
    };
    let result = match input {
        Ok(command) => medications::execute_with_options(
            &tenant,
            medications::Command::AdjustStock(command),
            None,
            Some(principal.provenance()),
        )
        .await
        .map_err(response::operation),
        Err(error) => Err(error),
    };
    let result = match result {
        Ok(record) => medications::read_stock_snapshot(&tenant, &record.id.to_string())
            .await
            .map(|snapshot| (StatusCode::OK, snapshot.representation, Some(snapshot.etag)))
            .map_err(response::operation),
        Err(error) => Err(error),
    };
    let savepoint_result = if result.is_ok() {
        savepoint.commit().await
    } else {
        savepoint.rollback().await
    };
    if savepoint_result.is_err() {
        return response::error(response::unavailable(), &request_id);
    }
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::stock(),
        result,
        &request_id,
    )
    .await
}

type Reply = std::result::Result<(StatusCode, Value, Option<String>), response::Failure>;

async fn finish(
    tenant: TenantTransaction,
    provenance: &CredentialProvenance,
    request: audit::RequestAudit,
    reply: Reply,
    request_id: &str,
) -> Response {
    let status = match &reply {
        Ok((status, _, _)) => *status,
        Err(error) => error.status,
    };
    if audit::record(&tenant, provenance, request, status)
        .await
        .is_err()
        || tenant.commit().await.is_err()
    {
        return response::error(response::unavailable(), request_id);
    }
    match reply {
        Ok((status, body, etag)) => response::success(status, body, request_id, etag),
        Err(error) => response::error(error, request_id),
    }
}
