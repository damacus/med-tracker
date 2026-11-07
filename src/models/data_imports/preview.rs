use std::collections::HashSet;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use loco_rs::app::AppContext;
use sea_orm::EntityTrait;

use super::{
    consents,
    contracts::{
        ConsentInput, ImportIssue, ImportState, PersonMapping, PreviewResult, PreviewToken,
        ReasonCode, RecordKind, SourceTimeZone,
    },
    credential::ImportPrincipal,
    identity::{self, IdentityChoices, IdentityValidationError, RecipientConsent},
    inspection::{self, InspectionError, Upload},
    persistence::{self, EncryptedPrepared, NewRun, PreviewContext},
    plan_projection::{self, BridgeIssue, PersonFinding},
    projection::{self, ProjectionMeta},
};
use crate::models::{
    access::{self, HouseholdScope},
    care::portable_imports::{
        AliasError, IncompatibleField,
        planning::{self, ExpectationLock},
    },
    entities::account,
    errors::OperationError,
};

pub struct PreviewInput {
    pub inspection_id: String,
    pub upload: Upload,
    pub timezone: SourceTimeZone,
    pub mappings: Vec<PersonMapping>,
    pub declarations: Vec<ConsentInput>,
    pub recipient_consents: Vec<RecipientConsent>,
    pub acknowledged_omissions: Vec<String>,
}

pub async fn create(
    ctx: &AppContext,
    scope: &HouseholdScope,
    input: PreviewInput,
) -> Result<PreviewResult, InspectionError> {
    create_inner(ctx, scope, input, None).await
}

pub async fn create_checked(
    ctx: &AppContext,
    scope: &HouseholdScope,
    input: PreviewInput,
    principal: ImportPrincipal<'_>,
) -> Result<PreviewResult, InspectionError> {
    create_inner(ctx, scope, input, Some(principal)).await
}

async fn create_inner(
    ctx: &AppContext,
    scope: &HouseholdScope,
    input: PreviewInput,
    principal: Option<ImportPrincipal<'_>>,
) -> Result<PreviewResult, InspectionError> {
    let PreviewInput {
        inspection_id,
        upload,
        timezone,
        mappings,
        declarations,
        recipient_consents,
        acknowledged_omissions,
    } = input;
    let source = upload.source;
    let (prepared, payload) = inspection::prepare_assessment(ctx, upload).await?;
    let required: HashSet<_> = prepared
        .inventory
        .omissions
        .iter()
        .map(|omission| omission.acknowledgement_id.as_str())
        .collect();
    let mut accepted = HashSet::new();
    if required.len() != prepared.inventory.omissions.len()
        || acknowledged_omissions
            .iter()
            .any(|id| !required.contains(id.as_str()) || !accepted.insert(id.as_str()))
    {
        return Err(OperationError::Forbidden.into());
    }
    let ids: Vec<_> = recipient_consents
        .iter()
        .map(|item| item.request_id.clone())
        .collect();
    let tenant =
        consents::begin_for_requests(&ctx.db, scope, ctx.config.settings.as_ref(), &ids).await?;
    if let Some(principal) = principal {
        principal.revalidate(&tenant).await?;
    }
    inspection::verify(&tenant, &inspection_id, &prepared.archive_sha256, timezone).await?;
    let actor = account::Entity::find_by_id(scope.actor.account_id)
        .one(tenant.transaction())
        .await
        .map_err(OperationError::from)?
        .ok_or(OperationError::Unauthenticated)?;
    let zone = crate::models::identity::time_zone::preferred(&actor.preferences, tenant.settings())
        .map_err(|_| OperationError::Unavailable)?;
    let today = Utc::now().with_timezone(&zone).date_naive();
    let choices = IdentityChoices {
        mappings: &mappings,
        declarations: &declarations,
        recipient_consents: &recipient_consents,
    };
    let (bridge, expectation, planner_blocked) =
        match identity::validate(&tenant, &prepared, payload, timezone, choices, today).await {
            Ok(validated) => {
                let mut findings = Vec::new();
                for person in &validated.people {
                    if !person.declaration_satisfied {
                        findings.push(PersonFinding {
                            person: person.source.clone(),
                            field: None,
                            reason: ReasonCode::GuardianDeclarationRequired,
                            detail: "A current guardian declaration is required.".into(),
                        });
                    } else if !person.consent_current {
                        findings.push(PersonFinding {
                            person: person.source.clone(),
                            field: None,
                            reason: ReasonCode::ConsentRequired,
                            detail: "Current consent is required.".into(),
                        });
                    }
                }
                let assessment = planning::assess(
                    &validated.authority,
                    validated.normalized,
                    &validated.declarations,
                    today,
                    ExpectationLock::Confirmation,
                )
                .await?;
                let blocked = assessment.has_blockers || !assessment.source_compatible;
                let bridge =
                    plan_projection::bridge(&prepared.inventory, &assessment, &findings, &[])
                        .map_err(|_| OperationError::unavailable("import_bridge_invalid"))?;
                (bridge, Some(assessment.expectation), blocked)
            }
            Err(IdentityValidationError::MissingDeclarations(people)) => {
                let findings: Vec<_> = people
                    .into_iter()
                    .map(|person| PersonFinding {
                        person,
                        field: None,
                        reason: ReasonCode::GuardianDeclarationRequired,
                        detail: "A current guardian declaration is required.".into(),
                    })
                    .collect();
                let bridge = plan_projection::blocked_inventory(
                    &prepared.inventory,
                    &findings,
                    &[],
                    BridgeIssue {
                        source: None,
                        field: None,
                        reason: ReasonCode::GuardianDeclarationRequired,
                        detail: "Identity assessment could not be completed.".into(),
                    },
                )
                .map_err(|_| OperationError::unavailable("import_bridge_invalid"))?;
                (bridge, None, true)
            }
            Err(IdentityValidationError::Alias(AliasError::IncompatiblePerson {
                source_person_portable_id,
                field,
                ..
            })) => {
                let person = prepared
                    .inventory
                    .records
                    .iter()
                    .find(|record| {
                        record.identity.kind == RecordKind::Person
                            && record.identity.id == source_person_portable_id
                    })
                    .ok_or_else(|| OperationError::unavailable("import_identity_source_missing"))?
                    .identity
                    .clone();
                let field = match field {
                    IncompatibleField::DateOfBirth => "date_of_birth",
                    IncompatibleField::PersonType => "person_type",
                    IncompatibleField::HasCapacity => "has_capacity",
                };
                let findings = [PersonFinding {
                    person,
                    field: Some(field.into()),
                    reason: ReasonCode::IdentityConflict,
                    detail: "The source and destination profiles are incompatible.".into(),
                }];
                let bridge = plan_projection::blocked_inventory(
                    &prepared.inventory,
                    &findings,
                    &[],
                    BridgeIssue {
                        source: None,
                        field: None,
                        reason: ReasonCode::IdentityConflict,
                        detail: "Identity assessment could not be completed.".into(),
                    },
                )
                .map_err(|_| OperationError::unavailable("import_bridge_invalid"))?;
                (bridge, None, true)
            }
            Err(
                IdentityValidationError::Operation(error)
                | IdentityValidationError::Alias(AliasError::Database(error)),
            ) => return Err(error.into()),
            Err(IdentityValidationError::Alias(
                AliasError::InvalidPayload | AliasError::InvalidSelection,
            )) => return Err(OperationError::Forbidden.into()),
        };
    let mut report = projection::project(
        ProjectionMeta {
            id: uuid::Uuid::new_v4().to_string(),
            source,
            source_sha256: prepared.archive_sha256.clone(),
            timezone,
            created_at: Utc::now(),
        },
        &prepared.inventory,
        bridge.outcomes,
        &acknowledged_omissions,
    )?;
    report.issues = bridge
        .issues
        .into_iter()
        .map(|issue| ImportIssue {
            source: issue.source,
            field: issue.field,
            reason: issue.reason,
            detail: issue.detail,
        })
        .collect();
    let ready = !planner_blocked
        && report.issues.is_empty()
        && report.totals.decisions_required == 0
        && expectation.is_some();
    let confirmation = if ready {
        let mut bytes = [0_u8; 32];
        getrandom::fill(&mut bytes)
            .map_err(|_| OperationError::unavailable("import_token_random"))?;
        report.state = ImportState::Ready;
        Some(PreviewToken(URL_SAFE_NO_PAD.encode(bytes)))
    } else {
        None
    };
    let encrypted_prepared = EncryptedPrepared::from_prepared(&prepared.transfer)
        .map_err(|_| OperationError::unavailable("import_prepared_encode"))?;
    let run = NewRun {
        report: report.clone(),
        receipt: prepared.receipt,
        context: Some(PreviewContext::new(
            expectation,
            recipient_consents,
            Some(prepared.inventory.clone()),
            confirmation
                .as_ref()
                .map(super::result_projection::token_verifier),
        )),
        encrypted_prepared,
        source_timezone: timezone,
        mappings,
        consents: declarations,
        acknowledged_omissions,
        confirmation: confirmation.clone(),
    };
    access::recheck(&tenant).await?;
    persistence::create(&tenant, &run).await?;
    tenant.commit().await?;
    Ok(PreviewResult {
        report,
        confirmation,
    })
}
