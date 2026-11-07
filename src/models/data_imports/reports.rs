use super::{
    consents::{self, ConsentBinding},
    contracts::{
        ActualRecord, DestinationId, ImportReport, ImportState, PersonDestination, PreviewResult,
    },
    credential::ImportPrincipal,
    persistence, result_projection,
};
use crate::models::{
    _entities::{data_import_consents, data_import_runs},
    access::{self, HouseholdScope, PersonAccess, TenantTransaction},
    care::{
        guardian_declarations,
        portable_imports::{
            PreparedImportAuthority,
            planning::{self, ExpectationLock},
        },
    },
    entities::{account, person},
    errors::OperationError,
    ids::PersonId,
};
use chrono::Utc;
use loco_rs::app::AppContext;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

pub struct ReportPage {
    pub reports: Vec<ImportReport>,
    pub next_cursor: Option<String>,
}

fn request_ids(stored: &persistence::StoredRun) -> Vec<String> {
    stored
        .snapshot
        .context
        .as_ref()
        .map_or_else(Vec::new, |context| {
            context
                .recipient_consents()
                .iter()
                .map(|item| item.request_id.clone())
                .collect()
        })
}

fn immutable_matches(a: &persistence::StoredRun, b: &persistence::StoredRun) -> bool {
    a.created_by_membership_id == b.created_by_membership_id
        && a.snapshot.receipt == b.snapshot.receipt
        && a.snapshot.context == b.snapshot.context
        && a.snapshot.encrypted_prepared == b.snapshot.encrypted_prepared
        && a.snapshot.source_timezone == b.snapshot.source_timezone
        && a.snapshot.mappings == b.snapshot.mappings
        && a.snapshot.consents == b.snapshot.consents
        && a.snapshot.acknowledged_omissions == b.snapshot.acknowledged_omissions
        && a.snapshot.report.id == b.snapshot.report.id
        && a.snapshot.report.source == b.snapshot.report.source
        && a.snapshot.report.source_sha256 == b.snapshot.report.source_sha256
        && a.snapshot.report.created_at == b.snapshot.report.created_at
        && a.snapshot.report.source_timezone == b.snapshot.report.source_timezone
}

fn profile_only(report: &ImportReport, person: &super::contracts::PersonReport) -> bool {
    let source = &person.person.source;
    person.records.is_empty()
        && person.relationships.is_empty()
        && report.records.is_empty()
        && report.relationships.is_empty()
        && !report
            .people
            .iter()
            .flat_map(|row| &row.relationships)
            .any(|row| &row.source == source || &row.target == source)
}

fn subject_id(
    state: ImportState,
    mapping: &super::contracts::PersonMapping,
    person: &super::contracts::PersonReport,
) -> Option<Option<i64>> {
    match &mapping.destination {
        PersonDestination::Existing(id) => {
            if state == ImportState::Completed
                && !matches!(&person.person.destination, Some(destination) if destination.id == DestinationId::Person(*id))
            {
                return None;
            }
            Some(Some(id.0))
        }
        PersonDestination::NewDependent if state == ImportState::Completed => {
            if person.person.actual != Some(ActualRecord::Created) {
                return None;
            }
            match &person.person.destination {
                Some(destination) => match &destination.id {
                    DestinationId::Person(id) => Some(Some(id.0)),
                    _ => None,
                },
                None => None,
            }
        }
        PersonDestination::NewDependent => {
            if person.person.destination.is_some()
                || person.person.actual == Some(ActualRecord::Created)
            {
                None
            } else {
                Some(None)
            }
        }
    }
}

fn report_subjects(current: &persistence::StoredRun) -> Result<Vec<i64>, OperationError> {
    let report = &current.snapshot.report;
    let context = current
        .snapshot
        .context
        .as_ref()
        .ok_or(OperationError::NotFound)?;
    if context.archive_sha256() != Some(report.source_sha256.as_str())
        || context.inventory().is_none()
    {
        return Err(OperationError::NotFound);
    }
    let mut subjects = Vec::new();
    for item in &report.people {
        let mut mappings = current
            .snapshot
            .mappings
            .iter()
            .filter(|m| m.source == item.person.source);
        let mapping = mappings.next().ok_or(OperationError::NotFound)?;
        if mappings.next().is_some() {
            return Err(OperationError::NotFound);
        }
        let subject = subject_id(report.state, mapping, item).ok_or(OperationError::NotFound)?;
        if let Some(id) = subject {
            subjects.push(id);
        }
    }
    if current.snapshot.mappings.len() != report.people.len() {
        return Err(OperationError::NotFound);
    }
    if context.recipient_consents().iter().any(|entry| {
        current
            .snapshot
            .mappings
            .iter()
            .filter(|mapping| mapping.source == entry.source_person)
            .count()
            != 1
    }) {
        return Err(OperationError::NotFound);
    }
    Ok(subjects)
}

pub(crate) struct ReportDisclosureGuard<'a> {
    authority: PreparedImportAuthority<'a>,
    subjects: Vec<i64>,
    unavailable: Vec<i64>,
}

pub(crate) async fn prepare_report_disclosure<'a>(
    tenant: &'a TenantTransaction,
    runs: &[&persistence::StoredRun],
    today: chrono::NaiveDate,
) -> Result<ReportDisclosureGuard<'a>, OperationError> {
    let authority = PreparedImportAuthority::prepare(tenant, today).await?;
    let mut subjects = vec![tenant.membership().person_id];
    for run in runs {
        subjects.extend(report_subjects(run)?);
    }
    subjects.sort_unstable();
    subjects.dedup();
    let mut unavailable = Vec::new();
    for id in &subjects {
        if person::Entity::find_by_id(*id)
            .lock_shared()
            .one(tenant.transaction())
            .await?
            .is_none()
        {
            if *id == tenant.membership().person_id {
                return Err(OperationError::Forbidden);
            }
            unavailable.push(*id);
        }
    }
    Ok(ReportDisclosureGuard {
        authority,
        subjects,
        unavailable,
    })
}

pub(crate) async fn authorise_report_prepared(
    guard: &ReportDisclosureGuard<'_>,
    current: &persistence::StoredRun,
    today: chrono::NaiveDate,
) -> Result<(), OperationError> {
    let authority = &guard.authority;
    let tenant = authority.tenant();
    if current.created_by_membership_id != tenant.membership().id
        || !access::can_manage_household(tenant)
    {
        return Err(OperationError::NotFound);
    }
    authority.check_date(today)?;
    for id in report_subjects(current)? {
        if guard.subjects.binary_search(&id).is_err() {
            return Err(OperationError::Forbidden);
        }
        if guard.unavailable.binary_search(&id).is_ok() {
            return Err(OperationError::NotFound);
        }
    }
    let report = &current.snapshot.report;
    let context = current
        .snapshot
        .context
        .as_ref()
        .ok_or(OperationError::NotFound)?;
    let eligible = match guardian_declarations::eligible_actor(tenant, today).await {
        Ok(_) => true,
        Err(OperationError::Forbidden) => false,
        Err(error) => return Err(error),
    };
    for item in &report.people {
        let mapping = current
            .snapshot
            .mappings
            .iter()
            .find(|mapping| mapping.source == item.person.source)
            .ok_or(OperationError::NotFound)?;
        let subject = subject_id(report.state, mapping, item).ok_or(OperationError::NotFound)?;
        let Some(subject) = subject else {
            if context
                .recipient_consents()
                .iter()
                .any(|entry| entry.source_person == mapping.source)
            {
                return Err(OperationError::NotFound);
            }
            if !eligible {
                return Err(OperationError::NotFound);
            }
            continue;
        };
        let mut matching = context
            .recipient_consents()
            .iter()
            .filter(|entry| entry.source_person == mapping.source);
        let recipient = matching.next();
        if matching.next().is_some() {
            return Err(OperationError::NotFound);
        }
        if let Some(recipient) = recipient {
            let PersonDestination::Existing(target) = &mapping.destination else {
                return Err(OperationError::NotFound);
            };
            let binding = ConsentBinding {
                source_sha256: report.source_sha256.clone(),
                source_person: mapping.source.clone(),
                target_person: *target,
            };
            match consents::current(tenant, &recipient.request_id, &binding).await {
                Ok(true) => (),
                Ok(false)
                | Err(
                    OperationError::NotFound
                    | OperationError::Forbidden
                    | OperationError::Unauthenticated,
                ) => return Err(OperationError::NotFound),
                Err(error) => return Err(error),
            }
        }
        if (subject == tenant.membership().person_id || eligible)
            && authority.allows(PersonId::from_database(subject)).await?
        {
            continue;
        }
        if !profile_only(report, item)
            || subject == tenant.membership().person_id
            || recipient.is_none()
        {
            return Err(OperationError::NotFound);
        }
    }
    access::recheck(tenant).await
}

async fn report_zone(tenant: &TenantTransaction) -> Result<chrono_tz::Tz, OperationError> {
    let actor = account::Entity::find_by_id(tenant.scope().actor.account_id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    crate::models::identity::time_zone::preferred(&actor.preferences, tenant.settings())
        .map_err(|_| OperationError::Unavailable)
}

async fn checked_detail_inner(
    ctx: &AppContext,
    scope: &HouseholdScope,
    id: &str,
    principal: &ImportPrincipal<'_>,
) -> Result<PreviewResult, OperationError> {
    let lookup = access::begin(&ctx.db, scope, ctx.config.settings.as_ref()).await?;
    access::recheck(&lookup).await?;
    if !access::can_manage_household(&lookup) {
        return Err(OperationError::Forbidden);
    }
    let initial = persistence::find(&lookup, id)
        .await?
        .ok_or(OperationError::NotFound)?;
    let ids = request_ids(&initial);
    lookup.rollback().await?;
    let tenant =
        consents::begin_for_requests(&ctx.db, scope, ctx.config.settings.as_ref(), &ids).await?;
    principal.revalidate(&tenant).await?;
    let current = persistence::find_locked(&tenant, id)
        .await?
        .ok_or(OperationError::NotFound)?;
    if !immutable_matches(&initial, &current) {
        return Err(OperationError::NotFound);
    }
    let zone = report_zone(&tenant).await?;
    let today = Utc::now().with_timezone(&zone).date_naive();
    let authority = prepare_report_disclosure(&tenant, &[&current], today).await?;
    authorise_report_prepared(&authority, &current, today).await?;
    let report = &current.snapshot.report;
    let confirmation = if report.state == ImportState::Ready {
        match (&current.snapshot.context, &current.snapshot.confirmation) {
            (Some(context), Some(token))
                if context.archive_sha256() == Some(report.source_sha256.as_str())
                    && context.eligibility_timezone() == Some(zone.name())
                    && context.inventory().is_some()
                    && context.token_verifier().is_some_and(|verifier| {
                        result_projection::token_matches(verifier, token)
                    }) =>
            {
                if let Some(expected) = context.expectation() {
                    if planning::check_expectation_prepared(
                        &authority.authority,
                        expected,
                        ExpectationLock::Confirmation,
                        today,
                    )
                    .await?
                    {
                        Some(token.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    } else {
        None
    };
    access::recheck(&tenant).await?;
    principal.revalidate(&tenant).await?;
    authority
        .authority
        .check_date(Utc::now().with_timezone(&zone).date_naive())?;
    let report = current.snapshot.report;
    tenant.commit().await?;
    Ok(PreviewResult {
        report,
        confirmation,
    })
}

pub async fn detail_checked(
    ctx: &AppContext,
    scope: &HouseholdScope,
    id: &str,
    principal: ImportPrincipal<'_>,
) -> Result<PreviewResult, OperationError> {
    checked_detail_inner(ctx, scope, id, &principal).await
}

pub async fn list_checked(
    ctx: &AppContext,
    scope: &HouseholdScope,
    after_id: Option<&str>,
    limit: u64,
    principal: ImportPrincipal<'_>,
) -> Result<ReportPage, OperationError> {
    if !(1..=100).contains(&limit) {
        return Err(OperationError::Validation {
            details: serde_json::json!({"errors":{"limit":["must be between 1 and 100"]}}),
        });
    }
    let lookup = access::begin(&ctx.db, scope, ctx.config.settings.as_ref()).await?;
    access::recheck(&lookup).await?;
    if !access::can_manage_household(&lookup) {
        return Err(OperationError::Forbidden);
    }
    let mut query = data_import_runs::Entity::find()
        .select_only()
        .column(data_import_runs::Column::Id)
        .filter(data_import_runs::Column::HouseholdId.eq(scope.household_id))
        .filter(data_import_runs::Column::CreatedByMembershipId.eq(lookup.membership().id))
        .order_by_asc(data_import_runs::Column::Id)
        .limit(limit);
    if let Some(id) = after_id {
        query = query.filter(data_import_runs::Column::Id.gt(id.to_owned()));
    }
    let ids: Vec<String> = query
        .into_tuple::<(String,)>()
        .all(lookup.transaction())
        .await?
        .into_iter()
        .map(|(id,)| id)
        .collect();
    let next_cursor =
        if ids.len() == usize::try_from(limit).map_err(|_| OperationError::Unavailable)? {
            ids.last().cloned()
        } else {
            None
        };
    let mut initial = Vec::new();
    let mut requests = Vec::new();
    let mut visible_requests = std::collections::HashMap::new();
    for id in ids {
        if let Some(run) = persistence::find(&lookup, &id).await? {
            let ids = request_ids(&run);
            let mut available = true;
            for request in &ids {
                let visible = if let Some(visible) = visible_requests.get(request) {
                    *visible
                } else {
                    let visible = data_import_consents::Entity::find_by_id(request.clone())
                        .filter(
                            data_import_consents::Column::RequesterAccountId
                                .eq(scope.actor.account_id),
                        )
                        .filter(
                            data_import_consents::Column::RequesterMembershipId
                                .eq(lookup.membership().id),
                        )
                        .one(lookup.transaction())
                        .await?
                        .is_some();
                    visible_requests.insert(request.clone(), visible);
                    visible
                };
                available &= visible;
            }
            if available {
                requests.extend(ids);
                initial.push(run);
            }
        }
    }
    lookup.rollback().await?;
    requests.sort_unstable();
    requests.dedup();
    let tenant =
        consents::begin_for_requests(&ctx.db, scope, ctx.config.settings.as_ref(), &requests)
            .await?;
    principal.revalidate(&tenant).await?;
    let mut current = Vec::new();
    for run in &initial {
        if let Some(stored) = persistence::find_locked(&tenant, &run.snapshot.report.id).await?
            && immutable_matches(run, &stored)
            && report_subjects(&stored).is_ok()
        {
            current.push(stored);
        }
    }
    let zone = report_zone(&tenant).await?;
    let today = Utc::now().with_timezone(&zone).date_naive();
    let refs: Vec<_> = current.iter().collect();
    let authority = prepare_report_disclosure(&tenant, &refs, today).await?;
    let mut reports = Vec::new();
    for run in &current {
        match authorise_report_prepared(&authority, run, today).await {
            Ok(()) => reports.push(run.snapshot.report.clone()),
            Err(OperationError::NotFound) => (),
            Err(error) => return Err(error),
        }
    }
    access::recheck(&tenant).await?;
    principal.revalidate(&tenant).await?;
    authority
        .authority
        .check_date(Utc::now().with_timezone(&zone).date_naive())?;
    tenant.commit().await?;
    Ok(ReportPage {
        reports,
        next_cursor,
    })
}

async fn authorised(
    tenant: &TenantTransaction,
    stored: &persistence::StoredRun,
) -> Result<bool, OperationError> {
    access::recheck(tenant).await?;
    if !access::can_manage_household(tenant)
        || stored.created_by_membership_id != tenant.membership().id
    {
        return Ok(false);
    }
    for person in &stored.snapshot.report.people {
        let mut mappings = stored
            .snapshot
            .mappings
            .iter()
            .filter(|mapping| mapping.source == person.person.source);
        let Some(mapping) = mappings.next() else {
            return Ok(false);
        };
        if mappings.next().is_some() {
            return Ok(false);
        }
        if let PersonDestination::Existing(id) = &mapping.destination {
            match access::require_person_access(tenant, id.0, PersonAccess::Manage).await {
                Ok(()) => (),
                Err(OperationError::Forbidden) => return Ok(false),
                Err(error) => return Err(error),
            }
        }
    }
    access::recheck(tenant).await?;
    Ok(true)
}

pub async fn list(
    tenant: &TenantTransaction,
    after_id: Option<&str>,
    limit: u64,
) -> Result<ReportPage, OperationError> {
    access::recheck(tenant).await?;
    if !access::can_manage_household(tenant) {
        return Err(OperationError::Forbidden);
    }
    if !(1..=100).contains(&limit) {
        return Err(OperationError::Validation {
            details: serde_json::json!({"errors":{"limit":["must be between 1 and 100"]}}),
        });
    }
    let mut query = data_import_runs::Entity::find()
        .select_only()
        .column(data_import_runs::Column::Id)
        .filter(data_import_runs::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(data_import_runs::Column::CreatedByMembershipId.eq(tenant.membership().id))
        .order_by_asc(data_import_runs::Column::Id)
        .limit(limit);
    if let Some(id) = after_id {
        query = query.filter(data_import_runs::Column::Id.gt(id.to_owned()));
    }
    let ids: Vec<String> = query
        .into_tuple::<(String,)>()
        .all(tenant.transaction())
        .await?
        .into_iter()
        .map(|(id,)| id)
        .collect();
    let next_cursor =
        if ids.len() == usize::try_from(limit).map_err(|_| OperationError::Unavailable)? {
            ids.last().cloned()
        } else {
            None
        };
    let mut reports = Vec::new();
    for id in ids {
        if let Some(stored) = persistence::find(tenant, &id).await?
            && authorised(tenant, &stored).await?
        {
            reports.push(stored.snapshot.report);
        }
    }
    Ok(ReportPage {
        reports,
        next_cursor,
    })
}

pub async fn detail(tenant: &TenantTransaction, id: &str) -> Result<PreviewResult, OperationError> {
    let stored = persistence::find(tenant, id)
        .await?
        .ok_or(OperationError::NotFound)?;
    if !authorised(tenant, &stored).await? {
        return Err(OperationError::NotFound);
    }
    Ok(PreviewResult {
        report: stored.snapshot.report,
        confirmation: None,
    })
}
