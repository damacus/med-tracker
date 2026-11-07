use std::collections::HashSet;

use super::{
    consents::{self, ConsentBinding},
    contracts::{
        ConsentInput, PersonDestination, PersonMapping, PreparedArchive, RecordKind,
        SourceIdentity, SourceTimeZone,
    },
    inspection::{self, InspectionError, Upload},
};
use crate::models::{
    access::{self, HouseholdScope, TenantTransaction},
    care::{
        guardian_declarations,
        portable_imports::{
            self, AliasError, AliasSelection, ExistingIndex, InitialDeclarations, NormalizedImport,
            PreparedDeclarations, PreparedImportAuthority, ProfileDifference as AliasDifference,
        },
    },
    entities::{account, person},
    errors::OperationError,
    ids::{AccountId, PersonId},
};
use loco_rs::app::AppContext;
use sea_orm::{EntityTrait, QuerySelect};

#[derive(Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct RecipientConsent {
    pub source_person: SourceIdentity,
    pub request_id: String,
}

pub struct IdentityChoices<'a> {
    pub mappings: &'a [PersonMapping],
    pub declarations: &'a [ConsentInput],
    pub recipient_consents: &'a [RecipientConsent],
}

pub struct IdentityAssessment {
    pub archive_sha256: String,
    pub people: Vec<PersonAssessment>,
}

pub struct PersonAssessment {
    pub source: SourceIdentity,
    pub differences: Vec<ProfileDifference>,
    pub consent_current: bool,
    pub clinical_authority: bool,
    pub declaration_satisfied: bool,
}

pub enum ProfileDifference {
    Name {
        source: String,
        destination: String,
    },
    Email {
        source: Option<String>,
        destination: Option<String>,
    },
}

pub(crate) struct ValidatedIdentity<'a> {
    pub(crate) authority: PreparedImportAuthority<'a>,
    pub(crate) normalized: NormalizedImport,
    pub(crate) declarations: PreparedDeclarations,
    pub(crate) people: Vec<PersonAssessment>,
}

#[derive(Debug)]
pub(crate) enum IdentityValidationError {
    Alias(AliasError),
    MissingDeclarations(Vec<SourceIdentity>),
    Operation(OperationError),
}

impl From<OperationError> for IdentityValidationError {
    fn from(error: OperationError) -> Self {
        Self::Operation(error)
    }
}

pub(crate) async fn validate<'a>(
    tenant: &'a TenantTransaction,
    prepared: &PreparedArchive,
    payload: serde_json::Value,
    _timezone: SourceTimeZone,
    choices: IdentityChoices<'_>,
    today: chrono::NaiveDate,
) -> Result<ValidatedIdentity<'a>, IdentityValidationError> {
    let IdentityChoices {
        mappings,
        declarations,
        recipient_consents,
    } = choices;
    access::recheck(tenant).await?;
    if !access::can_manage_household(tenant)
        || !tenant
            .holds_exclusive_account(AccountId::from_database(tenant.scope().actor.account_id))
    {
        return Err(OperationError::Forbidden.into());
    }
    let source_people: HashSet<_> = prepared
        .inventory
        .records
        .iter()
        .filter(|record| record.identity.kind == RecordKind::Person)
        .map(|record| record.identity.clone())
        .collect();
    if source_people.len()
        != prepared
            .inventory
            .records
            .iter()
            .filter(|record| record.identity.kind == RecordKind::Person)
            .count()
    {
        return Err(OperationError::Forbidden.into());
    }
    let mut seen = HashSet::new();
    let mut targets = HashSet::new();
    let mut aliases = Vec::new();
    let mut new_dependents = Vec::new();
    for mapping in mappings {
        if !source_people.contains(&mapping.source) || !seen.insert(mapping.source.clone()) {
            return Err(OperationError::Forbidden.into());
        }
        match &mapping.destination {
            PersonDestination::Existing(id) => {
                if id.0 <= 0 || !targets.insert(id.0) {
                    return Err(OperationError::Forbidden.into());
                }
                aliases.push(AliasSelection {
                    source_person_portable_id: mapping.source.id.clone(),
                    destination_person_id: PersonId::from_database(id.0),
                });
            }
            PersonDestination::NewDependent => new_dependents.push(mapping.source.id.clone()),
        }
    }
    if seen != source_people {
        return Err(OperationError::Forbidden.into());
    }
    let mut consent_sources = HashSet::new();
    if recipient_consents.iter().any(|consent| {
        !source_people.contains(&consent.source_person)
            || !consent_sources.insert(consent.source_person.clone())
    }) {
        return Err(OperationError::Forbidden.into());
    }
    let mut declared = HashSet::new();
    for declaration in declarations {
        let key = match declaration {
            ConsentInput::ActingPerson { subject, .. } => {
                if subject.0 != tenant.membership().person_id || !targets.contains(&subject.0) {
                    return Err(OperationError::Forbidden.into());
                }
                (0_u8, Some(subject.0), None)
            }
            ConsentInput::ActingGuardian { subject, .. } => {
                if !targets.contains(&subject.0) || subject.0 == tenant.membership().person_id {
                    return Err(OperationError::Forbidden.into());
                }
                (1, Some(subject.0), None)
            }
            ConsentInput::InitialGuardianDeclaration { source_person, .. } => {
                if !mappings.iter().any(|mapping| {
                    mapping.source == *source_person
                        && matches!(&mapping.destination, PersonDestination::NewDependent)
                }) {
                    return Err(OperationError::Forbidden.into());
                }
                (2, None, Some(source_person.clone()))
            }
        };
        if !declared.insert(key) {
            return Err(OperationError::Forbidden.into());
        }
    }
    if recipient_consents.iter().any(|entry| !mappings.iter().any(|mapping| mapping.source == entry.source_person && matches!(&mapping.destination, PersonDestination::Existing(id) if id.0 != tenant.membership().person_id))) {
        return Err(OperationError::Forbidden.into());
    }
    let missing: Vec<_> = mappings.iter().filter(|mapping| matches!(&mapping.destination, PersonDestination::NewDependent) && !declarations.iter().any(|item| matches!(item, ConsentInput::InitialGuardianDeclaration { source_person, accepted: true } if *source_person == mapping.source))).map(|mapping| mapping.source.clone()).collect();
    if !missing.is_empty() {
        return Err(IdentityValidationError::MissingDeclarations(missing));
    }
    let authority = PreparedImportAuthority::prepare(tenant, today).await?;
    let mut target_ids: Vec<_> = targets.iter().copied().collect();
    target_ids.push(tenant.membership().person_id);
    target_ids.sort_unstable();
    target_ids.dedup();
    for id in target_ids {
        person::Entity::find_by_id(id)
            .lock_shared()
            .one(tenant.transaction())
            .await
            .map_err(OperationError::from)?
            .ok_or(OperationError::Forbidden)?;
    }
    access::recheck(tenant).await?;
    let normalized =
        portable_imports::normalize_aliases(tenant, payload, &aliases, &new_dependents)
            .await
            .map_err(IdentityValidationError::Alias)?;
    let attestations = declarations
        .iter()
        .filter_map(|item| match item {
            ConsentInput::InitialGuardianDeclaration {
                source_person,
                accepted: true,
            } => Some(source_person.id.clone()),
            _ => None,
        })
        .map(|id| {
            guardian_declarations::InitialGuardianAttestation::parse(Some(true))
                .map(|attestation| (id, attestation))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let initial = InitialDeclarations::from_attestations(attestations)?;
    let index = ExistingIndex::load(tenant.transaction(), tenant.scope().household_id).await?;
    let prepared_declarations = initial
        .prepare(tenant, &normalized.plan, &index, today)
        .await?;
    let mut people = Vec::new();
    for mapping in mappings {
        match &mapping.destination {
            PersonDestination::Existing(id) => {
                let subject = person::Entity::find_by_id(id.0)
                    .lock_shared()
                    .one(tenant.transaction())
                    .await
                    .map_err(OperationError::from)?
                    .ok_or(OperationError::Forbidden)?;
                let clinical_authority = authority.allows(PersonId::from_database(id.0)).await?;
                if declarations.iter().any(|item| matches!(item, ConsentInput::ActingGuardian { subject, accepted: true } if subject == id))
                    && ((subject.person_type == 0 && subject.has_capacity) || !clinical_authority)
                {
                    return Err(OperationError::Forbidden.into());
                }
                if recipient_consents
                    .iter()
                    .any(|entry| entry.source_person == mapping.source)
                    && !(subject.person_type == 0
                        && subject.has_capacity
                        && subject.id != tenant.membership().person_id)
                {
                    return Err(OperationError::Forbidden.into());
                }
                let consent_current = if subject.id == tenant.membership().person_id {
                    declarations.iter().any(|item| matches!(item, ConsentInput::ActingPerson { subject, accepted: true } if subject == id))
                } else if subject.person_type == 0 && subject.has_capacity {
                    if let Some(recipient) = recipient_consents
                        .iter()
                        .find(|entry| entry.source_person == mapping.source)
                    {
                        let binding = ConsentBinding {
                            source_sha256: prepared.archive_sha256.clone(),
                            source_person: mapping.source.clone(),
                            target_person: *id,
                        };
                        match consents::current(tenant, &recipient.request_id, &binding).await {
                            Ok(value) => value,
                            Err(
                                OperationError::Forbidden
                                | OperationError::NotFound
                                | OperationError::Unauthenticated,
                            ) => false,
                            Err(error) => return Err(error.into()),
                        }
                    } else {
                        false
                    }
                } else {
                    clinical_authority && declarations.iter().any(|item| matches!(item, ConsentInput::ActingGuardian { subject, accepted: true } if subject == id))
                };
                let differences = normalized
                    .aliases
                    .iter()
                    .find(|alias| alias.source_person_portable_id == mapping.source.id)
                    .ok_or(OperationError::Unavailable)?
                    .profile_differences
                    .iter()
                    .map(|difference| match difference {
                        AliasDifference::Name {
                            source,
                            destination,
                        } => ProfileDifference::Name {
                            source: source.clone(),
                            destination: destination.clone(),
                        },
                        AliasDifference::Email {
                            source,
                            destination,
                        } => ProfileDifference::Email {
                            source: source.clone(),
                            destination: destination.clone(),
                        },
                    })
                    .collect();
                people.push(PersonAssessment {
                    source: mapping.source.clone(),
                    differences,
                    consent_current,
                    clinical_authority,
                    declaration_satisfied: true,
                });
            }
            PersonDestination::NewDependent => {
                let eligible = match guardian_declarations::eligible_actor(tenant, today).await {
                    Ok(_) => true,
                    Err(OperationError::Forbidden) => false,
                    Err(error) => return Err(error.into()),
                };
                let declared = declarations.iter().any(|item| matches!(item, ConsentInput::InitialGuardianDeclaration { source_person, accepted: true } if *source_person == mapping.source));
                people.push(PersonAssessment {
                    source: mapping.source.clone(),
                    differences: Vec::new(),
                    consent_current: eligible && declared,
                    clinical_authority: false,
                    declaration_satisfied: eligible && declared,
                });
            }
        }
    }
    access::recheck(tenant).await?;
    Ok(ValidatedIdentity {
        authority,
        normalized,
        declarations: prepared_declarations,
        people,
    })
}

pub async fn assess(
    ctx: &AppContext,
    scope: &HouseholdScope,
    upload: Upload,
    inspection_id: &str,
    timezone: SourceTimeZone,
    choices: IdentityChoices<'_>,
) -> Result<IdentityAssessment, InspectionError> {
    let (prepared, payload) = inspection::prepare_assessment(ctx, upload).await?;
    let ids: Vec<_> = choices
        .recipient_consents
        .iter()
        .map(|consent| consent.request_id.clone())
        .collect();
    let tenant =
        consents::begin_for_requests(&ctx.db, scope, ctx.config.settings.as_ref(), &ids).await?;
    inspection::verify(&tenant, inspection_id, &prepared.archive_sha256, timezone).await?;
    let actor = account::Entity::find_by_id(scope.actor.account_id)
        .one(tenant.transaction())
        .await
        .map_err(OperationError::from)?
        .ok_or(OperationError::Unauthenticated)?;
    let zone = crate::models::identity::time_zone::preferred(&actor.preferences, tenant.settings())
        .map_err(|_| OperationError::Unavailable)?;
    let today = chrono::Utc::now().with_timezone(&zone).date_naive();
    let people = {
        let result = validate(&tenant, &prepared, payload, timezone, choices, today).await.map_err(|error| match error {
        IdentityValidationError::Operation(error) | IdentityValidationError::Alias(AliasError::Database(error)) => InspectionError::Operation(error),
        IdentityValidationError::Alias(AliasError::IncompatiblePerson { .. } | AliasError::InvalidPayload | AliasError::InvalidSelection) => InspectionError::Operation(OperationError::Conflict { code: "import_identity_conflict".into(), details: serde_json::json!({}) }),
        IdentityValidationError::MissingDeclarations(_) => InspectionError::Operation(OperationError::Validation { details: serde_json::json!({"errors":{"initial_guardian_declarations":["required for each new dependent"]}}) }),
    })?;
        result.people
    };
    tenant.commit().await?;
    Ok(IdentityAssessment {
        archive_sha256: prepared.archive_sha256,
        people,
    })
}
