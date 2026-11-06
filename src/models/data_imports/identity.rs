use std::collections::HashSet;

use super::{
    consents::{self, ConsentBinding},
    contracts::{
        ConsentInput, PersonDestination, PersonMapping, RecordKind, SourceIdentity, SourceTimeZone,
    },
    inspection::{self, InspectionError, Upload},
};
use crate::models::{
    access::{self, HouseholdScope, PersonAccess},
    care::{
        guardian_declarations,
        person_schedules::sharing,
        portable_imports::{
            self, AliasError, AliasSelection, ProfileDifference as AliasDifference,
        },
    },
    entities::person,
    errors::OperationError,
    ids::PersonId,
};
use loco_rs::app::AppContext;
use sea_orm::{EntityTrait, QuerySelect};

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

pub async fn assess(
    ctx: &AppContext,
    scope: &HouseholdScope,
    upload: Upload,
    inspection_id: &str,
    timezone: SourceTimeZone,
    choices: IdentityChoices<'_>,
) -> Result<IdentityAssessment, InspectionError> {
    let IdentityChoices {
        mappings,
        declarations,
        recipient_consents,
    } = choices;
    let (prepared, payload) = inspection::prepare_assessment(ctx, upload).await?;
    let source_people: HashSet<_> = prepared
        .inventory
        .records
        .iter()
        .filter(|record| record.identity.kind == RecordKind::Person)
        .map(|record| record.identity.clone())
        .collect();
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
    let tenant = access::begin(&ctx.db, scope, ctx.config.settings.as_ref()).await?;
    inspection::verify(&tenant, inspection_id, &prepared.archive_sha256, timezone).await?;
    let mut declared = HashSet::new();
    for declaration in declarations {
        let key = match declaration {
            ConsentInput::ActingPerson { subject, accepted } => {
                if !accepted
                    || subject.0 != tenant.membership().person_id
                    || !targets.contains(&subject.0)
                {
                    return Err(OperationError::Forbidden.into());
                }
                (0_u8, Some(subject.0), None)
            }
            ConsentInput::ActingGuardian { subject, accepted } => {
                if !accepted
                    || !targets.contains(&subject.0)
                    || subject.0 == tenant.membership().person_id
                {
                    return Err(OperationError::Forbidden.into());
                }
                (1, Some(subject.0), None)
            }
            ConsentInput::InitialGuardianDeclaration {
                source_person,
                accepted,
            } => {
                if !accepted
                    || !mappings.iter().any(|mapping| {
                        mapping.source == *source_person
                            && matches!(&mapping.destination, PersonDestination::NewDependent)
                    })
                {
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
    let normalized =
        portable_imports::normalize_aliases(&tenant, payload, &aliases, &new_dependents)
            .await
            .map_err(|error| match error {
                AliasError::Database(error) => InspectionError::Operation(error),
                AliasError::InvalidPayload
                | AliasError::InvalidSelection
                | AliasError::IncompatiblePerson => {
                    InspectionError::Operation(OperationError::Conflict {
                        code: "import_identity_conflict".into(),
                        details: serde_json::json!({}),
                    })
                }
            })?;
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
                let may_manage = match access::require_person_access(
                    &tenant,
                    id.0,
                    PersonAccess::Manage,
                )
                .await
                {
                    Ok(()) => true,
                    Err(OperationError::Forbidden) => false,
                    Err(error) => return Err(error.into()),
                };
                let clinical_authority = may_manage
                    && sharing::clinical_authority(
                        tenant.transaction(),
                        PersonId::from_database(id.0),
                    )
                    .await?;
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
                        match consents::current(&tenant, &recipient.request_id, &binding).await {
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
                let eligible = match guardian_declarations::eligible_actor(
                    &tenant,
                    chrono::Utc::now().date_naive(),
                )
                .await
                {
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
    access::recheck(&tenant).await?;
    tenant.commit().await?;
    Ok(IdentityAssessment {
        archive_sha256: prepared.archive_sha256,
        people,
    })
}
