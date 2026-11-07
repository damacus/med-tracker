use super::{
    consents::{self, ConsentBinding},
    contracts::{
        ActualRecord, DestinationId, ImportReport, ImportState, PersonDestination, PreviewResult,
    },
    credential::ImportPrincipal,
    persistence, result_projection,
};
use crate::models::{
    _entities::data_import_runs,
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

async fn checked_detail_inner(
    ctx: &AppContext,
    scope: &HouseholdScope,
    id: &str,
    principal: &ImportPrincipal<'_>,
    include_confirmation: bool,
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
    if !immutable_matches(&initial, &current)
        || current.created_by_membership_id != tenant.membership().id
        || !access::can_manage_household(&tenant)
    {
        return Err(OperationError::NotFound);
    }
    let actor = account::Entity::find_by_id(scope.actor.account_id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    let zone = crate::models::identity::time_zone::preferred(&actor.preferences, tenant.settings())
        .map_err(|_| OperationError::Unavailable)?;
    let today = Utc::now().with_timezone(&zone).date_naive();
    let authority = PreparedImportAuthority::prepare(&tenant, today).await?;
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
    let mut subjects = vec![tenant.membership().person_id];
    let mut resolved = Vec::with_capacity(report.people.len());
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
        resolved.push((item, mapping, subject));
    }
    if current.snapshot.mappings.len() != resolved.len() {
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
    subjects.sort_unstable();
    subjects.dedup();
    for id in subjects {
        person::Entity::find_by_id(id)
            .lock_shared()
            .one(tenant.transaction())
            .await?
            .ok_or(OperationError::NotFound)?;
    }
    let eligible = match guardian_declarations::eligible_actor(&tenant, today).await {
        Ok(_) => true,
        Err(OperationError::Forbidden) => false,
        Err(error) => return Err(error),
    };
    for (item, mapping, subject) in resolved {
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
            match consents::current(&tenant, &recipient.request_id, &binding).await {
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
    let confirmation = if include_confirmation && report.state == ImportState::Ready {
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
                        &authority,
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
    authority.check_date(Utc::now().with_timezone(&zone).date_naive())?;
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
    checked_detail_inner(ctx, scope, id, &principal, true).await
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
    let tenant = consents::begin_for_actor(&ctx.db, scope, ctx.config.settings.as_ref()).await?;
    principal.revalidate(&tenant).await?;
    if !access::can_manage_household(&tenant) {
        return Err(OperationError::Forbidden);
    }
    let mut query = data_import_runs::Entity::find()
        .select_only()
        .column(data_import_runs::Column::Id)
        .filter(data_import_runs::Column::HouseholdId.eq(scope.household_id))
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
    tenant.rollback().await?;
    let mut reports = Vec::new();
    for id in ids {
        match checked_detail_inner(ctx, scope, &id, &principal, false).await {
            Ok(result) => reports.push(result.report),
            Err(OperationError::NotFound) => (),
            Err(error) => return Err(error),
        }
    }
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
