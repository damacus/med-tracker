use super::{
    contracts::{ImportReport, PersonDestination, PreviewResult},
    persistence,
};
use crate::models::{
    _entities::data_import_runs,
    access::{self, PersonAccess, TenantTransaction},
    errors::OperationError,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

pub struct ReportPage {
    pub reports: Vec<ImportReport>,
    pub next_cursor: Option<String>,
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
