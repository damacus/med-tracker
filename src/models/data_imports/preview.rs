use std::collections::HashSet;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use loco_rs::app::AppContext;
use sea_orm::EntityTrait;

use super::{
    consents,
    contracts::{
        ConsentInput, ImportIssue, ImportReport, ImportState, PersonMapping, PreviewResult,
        PreviewToken, ReasonCode, RecordKind, SourceInventory, SourceTimeZone,
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

pub(crate) fn trace_preview_failure(stage: &'static str, error: &OperationError) {
    let category = match error {
        OperationError::Unauthenticated => "unauthenticated",
        OperationError::Forbidden => "forbidden",
        OperationError::NotFound => "not_found",
        OperationError::Validation { .. } => "validation",
        OperationError::Conflict { .. } => "conflict",
        OperationError::Unavailable => "unavailable",
    };
    tracing::warn!(
        medtracker.import.preview_stage = stage,
        medtracker.import.preview_category = category,
        "Clinical import preview stage failed"
    );
}

fn trace_inspection_failure(stage: &'static str, error: &InspectionError) {
    match error {
        InspectionError::Operation(error) => trace_preview_failure(stage, error),
        InspectionError::Preparation(_) => tracing::warn!(
            medtracker.import.preview_stage = stage,
            medtracker.import.preview_category = "preparation",
            "Clinical import preview stage failed"
        ),
    }
}

pub async fn create(
    ctx: &AppContext,
    scope: &HouseholdScope,
    input: PreviewInput,
) -> Result<PreviewResult, InspectionError> {
    create_inner(ctx, scope, input, None)
        .await
        .inspect_err(|error| trace_inspection_failure("preview", error))
}

pub async fn create_checked(
    ctx: &AppContext,
    scope: &HouseholdScope,
    input: PreviewInput,
    principal: ImportPrincipal<'_>,
) -> Result<PreviewResult, InspectionError> {
    create_inner(ctx, scope, input, Some(principal))
        .await
        .inspect_err(|error| trace_inspection_failure("preview", error))
}

fn source_only_blocked(
    report: &ImportReport,
    inventory: &SourceInventory,
    acknowledgements: &[String],
) -> Result<PreviewResult, OperationError> {
    let bridge = plan_projection::blocked_inventory(
        inventory,
        &[],
        &[],
        BridgeIssue {
            source: None,
            field: None,
            reason: ReasonCode::Other,
            detail: String::new(),
        },
    )
    .map_err(|_| OperationError::unavailable("import_source_projection_invalid"))?;
    let mut safe = projection::project(
        ProjectionMeta {
            id: report.id.clone(),
            source: report.source,
            source_sha256: report.source_sha256.clone(),
            timezone: report.source_timezone,
            created_at: report.created_at,
        },
        inventory,
        bridge.outcomes,
        acknowledgements,
    )?;
    let reasons: std::collections::HashMap<_, _> = report
        .people
        .iter()
        .flat_map(|person| std::iter::once(&person.person).chain(&person.records))
        .chain(&report.records)
        .filter_map(|record| {
            (record.expected == super::contracts::ExpectedRecord::Reject)
                .then(|| record.reason.map(|reason| (record.source.clone(), reason)))
                .flatten()
        })
        .collect();
    for record in safe
        .people
        .iter_mut()
        .flat_map(|person| std::iter::once(&mut person.person).chain(&mut person.records))
        .chain(&mut safe.records)
    {
        if let Some(reason) = reasons.get(&record.source) {
            record.reason = Some(*reason);
            record.expected = super::contracts::ExpectedRecord::Reject;
        }
    }
    safe.issues = report
        .issues
        .iter()
        .map(|issue| ImportIssue {
            source: issue.source.clone(),
            field: issue.field.clone(),
            reason: issue.reason,
            detail: match issue.reason {
                ReasonCode::ConsentRequired => "Current consent is required.",
                ReasonCode::GuardianDeclarationRequired => {
                    "A current guardian declaration is required."
                }
                ReasonCode::InsufficientAuthority => "Current authority is required.",
                ReasonCode::MissingDependency => "A source dependency is unavailable.",
                ReasonCode::IdentityConflict => {
                    "An identity or destination conflict requires review."
                }
                ReasonCode::SourceInvalid => "A source record is invalid.",
                ReasonCode::DestinationChanged => {
                    "The destination changed; review another preview."
                }
                _ => "Review this issue before continuing.",
            }
            .into(),
        })
        .collect();
    Ok(PreviewResult {
        report: safe,
        confirmation: None,
    })
}

async fn create_inner(
    ctx: &AppContext,
    scope: &HouseholdScope,
    input: PreviewInput,
    principal: Option<ImportPrincipal<'_>>,
) -> Result<PreviewResult, InspectionError> {
    let checked = principal.is_some();
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
    let (prepared, payload) = inspection::prepare_assessment(ctx, upload)
        .await
        .inspect_err(|error| trace_inspection_failure("prepare", error))?;
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
    let tenant = consents::begin_for_requests(&ctx.db, scope, ctx.config.settings.as_ref(), &ids)
        .await
        .inspect_err(|error| trace_preview_failure("begin", error))?;
    if let Some(principal) = principal {
        principal.revalidate(&tenant).await?;
    }
    inspection::verify(&tenant, &inspection_id, &prepared.archive_sha256, timezone)
        .await
        .inspect_err(|error| trace_preview_failure("inspection_binding", error))?;
    let actor = account::Entity::find_by_id(scope.actor.account_id)
        .one(tenant.transaction())
        .await
        .map_err(OperationError::from)?
        .ok_or(OperationError::Unauthenticated)?;
    let zone = crate::models::identity::time_zone::preferred(&actor.preferences, tenant.settings())
        .map_err(|_| OperationError::Unavailable)
        .inspect_err(|error| trace_preview_failure("actor_timezone", error))?;
    let today = Utc::now().with_timezone(&zone).date_naive();
    let choices = IdentityChoices {
        mappings: &mappings,
        declarations: &declarations,
        recipient_consents: &recipient_consents,
    };
    let (bridge, expectation, planner_blocked) = match identity::validate(
        &tenant, &prepared, payload, timezone, choices, today,
    )
    .await
    .inspect_err(|error| {
        if let IdentityValidationError::Operation(error)
        | IdentityValidationError::Alias(AliasError::Database(error)) = error
        {
            trace_preview_failure("identity", error);
        }
    }) {
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
            .await
            .inspect_err(|error| trace_preview_failure("planning", error))?;
            let blocked = assessment.has_blockers || !assessment.source_compatible;
            let bridge = plan_projection::bridge(&prepared.inventory, &assessment, &findings, &[])
                .map_err(|_| OperationError::unavailable("import_bridge_invalid"))
                .inspect_err(|error| trace_preview_failure("bridge", error))?;
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
            destination_person_id,
            field,
        })) => {
            if !mappings.iter().any(|mapping| {
                    mapping.source.id == source_person_portable_id
                        && matches!(&mapping.destination, super::contracts::PersonDestination::Existing(id) if id.0 == destination_person_id.get())
                }) {
                    return Err(OperationError::unavailable("import_identity_mapping_mismatch").into());
                }
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
    )
    .inspect_err(|error| trace_preview_failure("projection", error))?;
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
        .map_err(|_| OperationError::unavailable("import_prepared_encode"))
        .inspect_err(|error| trace_preview_failure("prepared_encode", error))?;
    let response = if checked && report.state == ImportState::Blocked {
        Some(source_only_blocked(
            &report,
            &prepared.inventory,
            &acknowledged_omissions,
        )?)
    } else {
        None
    };
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
            Some(prepared.archive_sha256.clone()),
            Some(zone.name().to_owned()),
        )),
        encrypted_prepared,
        source_timezone: timezone,
        mappings,
        consents: declarations,
        acknowledged_omissions,
        confirmation: confirmation.clone(),
    };
    access::recheck(&tenant)
        .await
        .inspect_err(|error| trace_preview_failure("final_access", error))?;
    persistence::create(&tenant, &run)
        .await
        .inspect_err(|error| trace_preview_failure("persistence", error))?;
    tenant
        .commit()
        .await
        .inspect_err(|error| trace_preview_failure("commit", error))?;
    Ok(response.unwrap_or(PreviewResult {
        report,
        confirmation,
    }))
}

#[cfg(test)]
mod disclosure_tests {
    use super::*;
    use crate::models::data_imports::contracts::{
        Omission, RecordDetails, SourceInventory, SourceRecord,
    };

    #[test]
    fn source_projection_keeps_blockers_without_destination_details() {
        let mut report: super::super::contracts::ImportReport =
            serde_json::from_str::<PreviewResult>(include_str!("examples/preview.json"))
                .unwrap()
                .report;
        report.state = ImportState::Blocked;
        let source = report.people[0].person.source.clone();
        let inventory = SourceInventory {
            records: vec![SourceRecord {
                identity: source.clone(),
                details: RecordDetails::default(),
            }],
            relationships: vec![],
            omissions: vec![Omission {
                item: source.clone(),
                field: Some("legacy_field".into()),
                reason: ReasonCode::UnsupportedField,
                acknowledgement_id: "source-omission".into(),
                acknowledged: false,
            }],
        };
        report.issues = vec![
            ImportIssue {
                source: Some(source.clone()),
                field: None,
                reason: ReasonCode::ConsentRequired,
                detail: "private destination name".into(),
            },
            ImportIssue {
                source: Some(source),
                field: Some("dependency".into()),
                reason: ReasonCode::MissingDependency,
                detail: "private destination id".into(),
            },
        ];
        let projected = source_only_blocked(&report, &inventory, &[]).unwrap();
        assert!(projected.confirmation.is_none());
        assert_eq!(projected.report.issues.len(), 2);
        assert_eq!(
            projected.report.issues[1].reason,
            ReasonCode::MissingDependency
        );
        assert!(projected.report.people[0].person.destination.is_none());
        assert_eq!(
            projected.report.people[0].omissions[0].acknowledgement_id,
            "source-omission"
        );
        assert!(!projected.report.people[0].omissions[0].acknowledged);
        let json = serde_json::to_string(&projected).unwrap();
        assert!(!json.contains("private destination"));
    }
}
