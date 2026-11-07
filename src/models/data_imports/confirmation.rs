use std::collections::HashSet;

use chrono::Utc;
use loco_rs::app::AppContext;
use sea_orm::{EntityTrait, TransactionTrait};

use super::{
    consents::{self, ConsentBinding},
    contracts::{DestinationId, ImportReport, ImportState, PersonDestination, PreviewToken},
    credential::ImportPrincipal,
    identity::{self, IdentityChoices, IdentityValidationError},
    inspection::{self, InspectionError},
    persistence::{self, NewRun, StoredRun},
    plan_projection, projection, result_projection,
};
use crate::models::{
    access::{self, HouseholdScope, TenantTransaction},
    care::portable_imports::{
        self, AliasError, ExistingIndex, PreparedImportAuthority,
        planning::{self, ExpectationLock},
    },
    entities::account,
    errors::OperationError,
    identity::credentials::CredentialProvenance,
    ids::PersonId,
};

fn stale() -> OperationError {
    OperationError::Conflict {
        code: "import_preview_changed".into(),
        details: serde_json::json!({}),
    }
}

fn immutable_matches(a: &NewRun, b: &NewRun) -> bool {
    a.receipt == b.receipt
        && a.context == b.context
        && a.encrypted_prepared == b.encrypted_prepared
        && a.source_timezone == b.source_timezone
        && a.mappings == b.mappings
        && a.consents == b.consents
        && a.acknowledged_omissions == b.acknowledged_omissions
        && a.report.id == b.report.id
        && a.report.source == b.report.source
        && a.report.source_sha256 == b.report.source_sha256
        && a.report.source_timezone == b.report.source_timezone
        && a.report.created_at == b.report.created_at
}

fn verify_binding(run: &StoredRun, token: &PreviewToken) -> Result<(), OperationError> {
    let snapshot = &run.snapshot;
    let context = snapshot.context.as_ref().ok_or_else(stale)?;
    let verifier = context.token_verifier().ok_or_else(stale)?;
    if !result_projection::token_matches(verifier, token) {
        return Err(OperationError::Forbidden);
    }
    if context.archive_sha256() != Some(snapshot.report.source_sha256.as_str())
        || context.inventory().is_none()
        || context.expectation().is_none()
        || context.eligibility_timezone().is_none()
        || snapshot.report.source_timezone != snapshot.source_timezone
    {
        return Err(stale());
    }
    let inventory = context.inventory().ok_or_else(stale)?;
    let required: HashSet<_> = inventory
        .omissions
        .iter()
        .map(|item| item.acknowledgement_id.as_str())
        .collect();
    let accepted: HashSet<_> = snapshot
        .acknowledged_omissions
        .iter()
        .map(String::as_str)
        .collect();
    if required.len() != inventory.omissions.len()
        || accepted.len() != snapshot.acknowledged_omissions.len()
        || required != accepted
        || inventory
            .records
            .iter()
            .any(|row| row.identity.source != snapshot.report.source)
    {
        return Err(stale());
    }
    Ok(())
}

async fn eligibility_zone(
    tenant: &TenantTransaction,
    snapshot: &NewRun,
) -> Result<chrono_tz::Tz, OperationError> {
    let actor = account::Entity::find_by_id(tenant.scope().actor.account_id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    let zone = crate::models::identity::time_zone::preferred(&actor.preferences, tenant.settings())
        .map_err(|_| OperationError::Unavailable)?;
    if snapshot
        .context
        .as_ref()
        .and_then(|context| context.eligibility_timezone())
        != Some(zone.name())
    {
        return Err(stale());
    }
    Ok(zone)
}

async fn authorise_replay(
    tenant: &TenantTransaction,
    snapshot: &NewRun,
    zone: chrono_tz::Tz,
) -> Result<(), OperationError> {
    let today = Utc::now().with_timezone(&zone).date_naive();
    let authority = PreparedImportAuthority::prepare(tenant, today).await?;
    let context = snapshot.context.as_ref().ok_or_else(stale)?;
    for person in &snapshot.report.people {
        let Some(destination) = &person.person.destination else {
            return Err(OperationError::Forbidden);
        };
        let DestinationId::Person(subject) = &destination.id else {
            return Err(OperationError::Forbidden);
        };
        if authority.allows(PersonId::from_database(subject.0)).await? {
            continue;
        }
        if !person.records.is_empty() || !person.relationships.is_empty() {
            return Err(OperationError::Forbidden);
        }
        let mapping = snapshot
            .mappings
            .iter()
            .find(|item| item.source == person.person.source)
            .ok_or(OperationError::Forbidden)?;
        if !matches!(&mapping.destination, PersonDestination::Existing(id) if id == subject) {
            return Err(OperationError::Forbidden);
        }
        let recipient = context
            .recipient_consents()
            .iter()
            .find(|item| item.source_person == mapping.source)
            .ok_or(OperationError::Forbidden)?;
        let binding = ConsentBinding {
            source_sha256: snapshot.report.source_sha256.clone(),
            source_person: mapping.source.clone(),
            target_person: *subject,
        };
        if !consents::current(tenant, &recipient.request_id, &binding).await? {
            return Err(OperationError::Forbidden);
        }
    }
    access::recheck(tenant).await
}

pub async fn confirm(
    ctx: &AppContext,
    scope: &HouseholdScope,
    id: &str,
    token: PreviewToken,
    passphrase: String,
    provenance: &CredentialProvenance,
    request_id: &str,
) -> Result<ImportReport, InspectionError> {
    confirm_inner(
        ctx, scope, id, token, passphrase, provenance, request_id, None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn confirm_checked(
    ctx: &AppContext,
    scope: &HouseholdScope,
    id: &str,
    token: PreviewToken,
    passphrase: String,
    provenance: &CredentialProvenance,
    request_id: &str,
    principal: ImportPrincipal<'_>,
) -> Result<ImportReport, InspectionError> {
    let actual = principal.provenance().clone();
    if std::mem::discriminant(&actual.method) != std::mem::discriminant(&provenance.method)
        || actual.reference != provenance.reference
    {
        return Err(OperationError::Forbidden.into());
    }
    confirm_inner(
        ctx,
        scope,
        id,
        token,
        passphrase,
        &actual,
        request_id,
        Some(principal),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn confirm_inner(
    ctx: &AppContext,
    scope: &HouseholdScope,
    id: &str,
    token: PreviewToken,
    passphrase: String,
    provenance: &CredentialProvenance,
    request_id: &str,
    principal: Option<ImportPrincipal<'_>>,
) -> Result<ImportReport, InspectionError> {
    if request_id != scope.request_id {
        return Err(OperationError::Forbidden.into());
    }
    let lookup = access::begin(&ctx.db, scope, ctx.config.settings.as_ref()).await?;
    access::recheck(&lookup).await?;
    if !access::can_manage_household(&lookup) {
        return Err(OperationError::Forbidden.into());
    }
    let initial = persistence::find(&lookup, id)
        .await?
        .ok_or(OperationError::NotFound)?;
    if initial.created_by_membership_id != lookup.membership().id {
        return Err(OperationError::NotFound.into());
    }
    verify_binding(&initial, &token)?;
    lookup.rollback().await?;
    let payload = inspection::decrypt_prepared(
        ctx,
        initial.snapshot.encrypted_prepared.clone(),
        initial.snapshot.receipt.target_sha256.clone(),
        passphrase,
    )
    .await?;
    let context = initial.snapshot.context.as_ref().ok_or_else(stale)?;
    let request_ids: Vec<_> = context
        .recipient_consents()
        .iter()
        .map(|item| item.request_id.clone())
        .collect();
    let tenant =
        consents::begin_for_requests(&ctx.db, scope, ctx.config.settings.as_ref(), &request_ids)
            .await?;
    if let Some(principal) = principal.as_ref() {
        principal.revalidate(&tenant).await?;
    }
    let current = persistence::find_locked(&tenant, id)
        .await?
        .ok_or(OperationError::NotFound)?;
    verify_binding(&current, &token)?;
    if !immutable_matches(&initial.snapshot, &current.snapshot) {
        return Err(stale().into());
    }
    access::recheck(&tenant).await?;
    let zone = eligibility_zone(&tenant, &current.snapshot).await?;
    if current.snapshot.report.state == ImportState::Completed {
        authorise_replay(&tenant, &current.snapshot, zone).await?;
        let report = current.snapshot.report;
        tenant.commit().await?;
        return Ok(report);
    }
    if current.snapshot.report.state != ImportState::Ready
        || current.snapshot.confirmation.as_ref() != Some(&token)
        || initial.snapshot.report.state != ImportState::Ready
        || initial.snapshot.report != current.snapshot.report
    {
        return Err(stale().into());
    }
    let snapshot = &current.snapshot;
    let context = snapshot.context.as_ref().ok_or_else(stale)?;
    let inventory = context.inventory().ok_or_else(stale)?;
    let expected = context.expectation().ok_or_else(stale)?;
    let today = Utc::now().with_timezone(&zone).date_naive();
    let choices = IdentityChoices {
        mappings: &snapshot.mappings,
        declarations: &snapshot.consents,
        recipient_consents: context.recipient_consents(),
    };
    let validated = identity::validate_stored(
        &tenant,
        inventory,
        &snapshot.report.source_sha256,
        payload,
        snapshot.source_timezone,
        choices,
        today,
    )
    .await
    .map_err(|error| match error {
        IdentityValidationError::Operation(error)
        | IdentityValidationError::Alias(AliasError::Database(error)) => error,
        IdentityValidationError::Alias(_) | IdentityValidationError::MissingDeclarations(_) => {
            stale()
        }
    })?;
    if validated
        .people
        .iter()
        .any(|person| !person.consent_current || !person.declaration_satisfied)
    {
        return Err(stale().into());
    }
    if !planning::check_expectation_prepared(
        &validated.authority,
        expected,
        ExpectationLock::Confirmation,
        today,
    )
    .await?
    {
        return Err(stale().into());
    }
    let assessment = planning::assess(
        &validated.authority,
        validated.normalized.clone(),
        &validated.declarations,
        today,
        ExpectationLock::Confirmation,
    )
    .await?;
    if assessment.has_blockers
        || !assessment.source_compatible
        || assessment.expectation != *expected
    {
        return Err(stale().into());
    }
    let bridge = plan_projection::bridge(inventory, &assessment, &[], &[]).map_err(|_| stale())?;
    if !bridge.issues.is_empty() {
        return Err(stale().into());
    }
    let mut rebuilt = projection::project(
        projection::ProjectionMeta {
            id: snapshot.report.id.clone(),
            source: snapshot.report.source,
            source_sha256: snapshot.report.source_sha256.clone(),
            timezone: snapshot.source_timezone,
            created_at: snapshot.report.created_at,
        },
        inventory,
        bridge.outcomes,
        &snapshot.acknowledged_omissions,
    )?;
    rebuilt.state = ImportState::Ready;
    if rebuilt != snapshot.report {
        return Err(stale().into());
    }
    access::recheck(&tenant).await?;
    let mut applying = snapshot.report.clone();
    applying.state = ImportState::Applying;
    persistence::transition(&tenant, ImportState::Ready, &applying, Some(&token)).await?;
    let child = tenant
        .transaction()
        .begin()
        .await
        .map_err(OperationError::from)?;
    let mut authority = validated.authority;
    let mut declarations = validated.declarations;
    let operation: Result<ImportReport, OperationError> = async {
        let mut index = ExistingIndex::load(tenant.transaction(), scope.household_id).await?;
        let created = portable_imports::create_only_records(
            &mut authority,
            &validated.normalized.plan,
            &mut index,
            &mut declarations,
            provenance,
            request_id,
            zone,
        )
        .await?;
        let canonical = portable_imports::initialize_canonical_schedules(
            &authority,
            &created,
            snapshot.source_timezone.zone(),
            zone,
            provenance,
        )
        .await?;
        let report = result_projection::completed(
            &tenant,
            &applying,
            inventory,
            &assessment.aliases,
            &index,
            &created,
            &canonical,
        )
        .await?;
        access::recheck(&tenant).await?;
        if eligibility_zone(&tenant, snapshot).await? != zone {
            return Err(stale());
        }
        authority.check_date(Utc::now().with_timezone(&zone).date_naive())?;
        Ok(report)
    }
    .await;
    let report = match operation {
        Ok(report) => {
            child.commit().await.map_err(OperationError::from)?;
            persistence::transition(&tenant, ImportState::Applying, &report, None).await?;
            report
        }
        Err(error) => {
            tracing::warn!(error.type = match error {
                OperationError::Unauthenticated => "unauthenticated",
                OperationError::Forbidden => "forbidden",
                OperationError::NotFound => "not_found",
                OperationError::Validation { .. } => "validation",
                OperationError::Conflict { .. } => "conflict",
                OperationError::Unavailable => "unavailable",
            }, "Clinical import apply failed");
            child.rollback().await.map_err(OperationError::from)?;
            let mut failed = applying;
            failed.state = ImportState::Failed;
            persistence::transition(&tenant, ImportState::Applying, &failed, None).await?;
            failed
        }
    };
    match tenant.commit().await {
        Ok(()) => Ok(report),
        Err(error) => {
            match reload_completed(
                ctx,
                scope,
                id,
                &token,
                &initial.snapshot,
                &request_ids,
                principal.as_ref(),
            )
            .await
            {
                Ok(Some(completed)) => Ok(completed),
                Ok(None) => Err(error.into()),
                Err(reload_error) => {
                    tracing::warn!(error = %reload_error, "Could not reload an uncertain import commit");
                    Err(error.into())
                }
            }
        }
    }
}

async fn reload_completed(
    ctx: &AppContext,
    scope: &HouseholdScope,
    id: &str,
    token: &PreviewToken,
    initial: &NewRun,
    request_ids: &[String],
    principal: Option<&ImportPrincipal<'_>>,
) -> Result<Option<ImportReport>, OperationError> {
    let tenant =
        consents::begin_for_requests(&ctx.db, scope, ctx.config.settings.as_ref(), request_ids)
            .await?;
    if let Some(principal) = principal {
        principal.revalidate(&tenant).await?;
    }
    let stored = persistence::find_locked(&tenant, id)
        .await?
        .ok_or(OperationError::NotFound)?;
    verify_binding(&stored, token)?;
    if !immutable_matches(initial, &stored.snapshot)
        || stored.snapshot.report.state != ImportState::Completed
    {
        tenant.rollback().await?;
        return Ok(None);
    }
    let zone = eligibility_zone(&tenant, &stored.snapshot).await?;
    authorise_replay(&tenant, &stored.snapshot, zone).await?;
    let report = stored.snapshot.report;
    tenant.commit().await?;
    Ok(Some(report))
}
