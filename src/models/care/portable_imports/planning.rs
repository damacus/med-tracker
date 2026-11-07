use super::{
    ExistingIndex, ImportPlan, NormalizedImport, PreparedDeclarations, ProfileDifference,
    ResolvedAlias, preflight,
};
use crate::models::{
    access::{self, TenantTransaction, person_queries},
    entities::{location, medication, membership, person},
    errors::OperationError,
    ids::{AccountId, HouseholdId, PersonId},
};
use chrono::NaiveDate;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SourceCollection {
    People,
    Locations,
    Medications,
    DosageOptions,
    Schedules,
    PersonMedications,
    MedicationTakes,
    NotificationPreferences,
    MedicationPausePeriods,
    DoseOccurrences,
    HealthEvents,
}

impl SourceCollection {
    const ALL: [Self; 11] = [
        Self::People,
        Self::Locations,
        Self::Medications,
        Self::DosageOptions,
        Self::Schedules,
        Self::PersonMedications,
        Self::MedicationTakes,
        Self::NotificationPreferences,
        Self::MedicationPausePeriods,
        Self::DoseOccurrences,
        Self::HealthEvents,
    ];

    const fn name(self) -> &'static str {
        match self {
            Self::People => "people",
            Self::Locations => "locations",
            Self::Medications => "medications",
            Self::DosageOptions => "dosage_options",
            Self::Schedules => "schedules",
            Self::PersonMedications => "person_medications",
            Self::MedicationTakes => "medication_takes",
            Self::NotificationPreferences => "notification_preferences",
            Self::MedicationPausePeriods => "medication_pause_periods",
            Self::DoseOccurrences => "dose_occurrences",
            Self::HealthEvents => "health_events",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|collection| collection.name() == name)
    }

    const fn conflict_field(self) -> Option<&'static str> {
        match self {
            Self::People => Some("email"),
            Self::Locations | Self::Medications => Some("name"),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ExpectationLock {
    None,
    Confirmation,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct DestinationExpectation {
    pub(crate) household_id: HouseholdId,
    pub(crate) actor_id: AccountId,
    pub(crate) actor_membership: membership::Model,
    pub(crate) aliases: Vec<AliasExpectation>,
    pub(crate) slots: Vec<SlotExpectation>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct AliasExpectation {
    pub(crate) source_person_portable_id: String,
    pub(crate) destination_person_id: PersonId,
    pub(crate) person: Option<person::Model>,
    pub(crate) active_membership: Option<membership::Model>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct SlotExpectation {
    pub(crate) collection: SourceCollection,
    pub(crate) portable_id: String,
    pub(crate) occupied: bool,
    pub(crate) name: Option<NameExpectation>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct NameExpectation {
    pub(crate) key: String,
    pub(crate) matching_portable_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PlannedAction {
    Match,
    Create,
    Reject,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AssessmentReason {
    OccupiedSourceId,
    NameConflict,
    InvalidSource,
    MissingDependency,
    IncompatiblePerson,
    InsufficientAuthority,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AssessmentIssue {
    pub(crate) collection: Option<SourceCollection>,
    pub(crate) portable_id: Option<String>,
    pub(crate) field: Option<String>,
    pub(crate) reason: AssessmentReason,
    pub(crate) detail: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlannedRow {
    pub(crate) collection: SourceCollection,
    pub(crate) portable_id: String,
    pub(crate) action: PlannedAction,
}

pub(crate) struct ImportAssessment {
    pub(crate) rows: Vec<PlannedRow>,
    pub(crate) issues: Vec<AssessmentIssue>,
    pub(crate) aliases: Vec<ResolvedAlias>,
    pub(crate) expectation: DestinationExpectation,
    pub(crate) source_compatible: bool,
    pub(crate) has_blockers: bool,
}

pub(crate) async fn assess(
    authority: &super::PreparedImportAuthority<'_>,
    mut normalized: NormalizedImport,
    declarations: &PreparedDeclarations,
    today: NaiveDate,
    lock: ExpectationLock,
) -> Result<ImportAssessment, OperationError> {
    authority.check_date(today)?;
    let tenant = authority.tenant();
    let expectation = capture_expectation_prepared(
        authority,
        &normalized.aliases,
        &normalized.plan,
        lock,
        today,
    )
    .await?;
    let index = ExistingIndex::load(tenant.transaction(), tenant.scope().household_id).await?;
    let preflight = preflight(tenant.transaction(), tenant, &normalized.plan, &index).await?;
    let mut rows: Vec<PlannedRow> = expectation
        .slots
        .iter()
        .map(|slot| PlannedRow {
            collection: slot.collection,
            portable_id: slot.portable_id.clone(),
            action: PlannedAction::Create,
        })
        .collect();
    rows.extend(normalized.aliases.iter().map(|alias| PlannedRow {
        collection: SourceCollection::People,
        portable_id: alias.source_person_portable_id.clone(),
        action: PlannedAction::Match,
    }));
    rows.sort_by(|a, b| (a.collection, &a.portable_id).cmp(&(b.collection, &b.portable_id)));
    let mut issues = Vec::new();
    for slot in &expectation.slots {
        if slot.occupied {
            issues.push(AssessmentIssue {
                collection: Some(slot.collection),
                portable_id: Some(slot.portable_id.clone()),
                field: Some("portable_id".into()),
                reason: AssessmentReason::OccupiedSourceId,
                detail: "The source portable ID is already present in the destination.".into(),
            });
        }
        if let Some(name) = &slot.name {
            for conflict in name
                .matching_portable_ids
                .iter()
                .filter(|id| *id != &slot.portable_id)
            {
                let field = slot.collection.conflict_field().unwrap_or("name");
                issues.push(AssessmentIssue {
                    collection: Some(slot.collection),
                    portable_id: Some(slot.portable_id.clone()),
                    field: Some(field.into()),
                    reason: AssessmentReason::NameConflict,
                    detail: format!("The {field} conflicts with portable ID {conflict}."),
                });
            }
        }
    }
    for conflict in &preflight.conflicts {
        if let Some(collection) = SourceCollection::from_name(conflict.record_type) {
            let issue = AssessmentIssue {
                collection: Some(collection),
                portable_id: Some(conflict.portable_id.clone()),
                field: Some(conflict.field.into()),
                reason: AssessmentReason::NameConflict,
                detail: format!(
                    "The {} conflicts with portable ID {}.",
                    conflict.field, conflict.existing_portable_id
                ),
            };
            if !issues.contains(&issue) {
                issues.push(issue);
            }
        }
    }
    for diagnostic in preflight.diagnostics(&normalized.plan, &normalized.source_payload) {
        let reason = match diagnostic.reason {
            super::preflight::DiagnosticReason::MissingDependency => {
                AssessmentReason::MissingDependency
            }
            super::preflight::DiagnosticReason::InvalidSource => AssessmentReason::InvalidSource,
        };
        issues.push(AssessmentIssue {
            collection: diagnostic
                .collection
                .as_deref()
                .and_then(SourceCollection::from_name),
            portable_id: diagnostic.portable_id,
            field: diagnostic.field,
            reason,
            detail: diagnostic.detail,
        });
    }
    issues.extend(missing_dependencies(&normalized.plan, &normalized.aliases));
    for alias in &mut normalized.aliases {
        let current = expectation
            .aliases
            .iter()
            .find(|item| item.source_person_portable_id == alias.source_person_portable_id);
        let source = normalized.source_payload["records"]["people"]
            .as_array()
            .and_then(|people| {
                people.iter().find(|row| {
                    row["portable_id"].as_str() == Some(alias.source_person_portable_id.as_str())
                })
            });
        let current_person = current.and_then(|item| item.person.as_ref());
        if let (Some(source), Some(person)) = (source, current_person) {
            let birth = source.get("date_of_birth").and_then(|value| {
                if value.is_null() {
                    Some(None)
                } else {
                    value
                        .as_str()
                        .and_then(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
                        .map(Some)
                }
            });
            let kind = match source["person_type"].as_str() {
                Some("adult") => Some(0),
                Some("minor") => Some(1),
                Some("dependent_adult") => Some(2),
                _ => None,
            };
            for (field, compatible) in [
                (
                    "portable_id",
                    person.portable_id == alias.destination_person_portable_id,
                ),
                ("date_of_birth", birth == Some(person.date_of_birth)),
                ("person_type", kind == Some(person.person_type)),
                (
                    "has_capacity",
                    source["has_capacity"].as_bool() == Some(person.has_capacity),
                ),
            ] {
                if !compatible {
                    issues.push(AssessmentIssue { collection: Some(SourceCollection::People), portable_id: Some(alias.source_person_portable_id.clone()), field: Some(field.into()), reason: AssessmentReason::IncompatiblePerson, detail: format!("The selected destination person's {field} no longer matches the source or selected identity.") });
                }
            }
            alias.profile_differences.clear();
            if let Some(name) = source["name"].as_str()
                && name != person.name
            {
                alias.profile_differences.push(ProfileDifference::Name {
                    source: name.to_owned(),
                    destination: person.name.clone(),
                });
            }
            let email = source["email"].as_str().map(str::to_owned);
            if email != person.email {
                alias.profile_differences.push(ProfileDifference::Email {
                    source: email,
                    destination: person.email.clone(),
                });
            }
        } else {
            issues.push(AssessmentIssue { collection: Some(SourceCollection::People), portable_id: Some(alias.source_person_portable_id.clone()), field: None, reason: AssessmentReason::IncompatiblePerson, detail: "The selected person is no longer an active household member or the source person is missing.".into() });
        }
    }
    let allowed =
        match super::access::authorized_prepared(authority, &normalized.plan, &index, declarations)
            .await
        {
            Ok(allowed) => allowed,
            Err(OperationError::Forbidden) => false,
            Err(error) => return Err(error),
        };
    if !allowed {
        issues.push(AssessmentIssue {
            collection: None,
            portable_id: None,
            field: None,
            reason: AssessmentReason::InsufficientAuthority,
            detail: "The current actor is not authorised for this import plan.".into(),
        });
    }
    for row in normalized.plan.rows("schedules") {
        let Some(source_id) = row["portable_id"].as_str() else {
            continue;
        };
        let Some(subject) = row["person_portable_id"].as_str() else {
            continue;
        };
        let permitted = if let Some(id) = index.get("people", subject) {
            authority.allows(PersonId::from_database(id)).await?
        } else {
            declarations.contains(subject)
        };
        if !permitted {
            issues.push(AssessmentIssue { collection: Some(SourceCollection::Schedules), portable_id: Some(source_id.to_owned()), field: Some("person_portable_id".into()), reason: AssessmentReason::InsufficientAuthority, detail: "The actor lacks current self or eligible guardian authority for this schedule's person.".into() });
        }
    }
    access::recheck(tenant).await?;
    Ok(finish_assessment(
        rows,
        issues,
        normalized.aliases,
        expectation,
    ))
}

fn missing_dependencies(plan: &ImportPlan, aliases: &[ResolvedAlias]) -> Vec<AssessmentIssue> {
    let mut issues = Vec::new();
    let mut check = |collection: SourceCollection,
                     row: &serde_json::Value,
                     field: &str,
                     target: SourceCollection,
                     id: &str| {
        let available = plan
            .rows(target.name())
            .iter()
            .any(|target_row| target_row["portable_id"].as_str() == Some(id))
            || (target == SourceCollection::People
                && aliases
                    .iter()
                    .any(|alias| alias.destination_person_portable_id == id));
        if !available {
            issues.push(AssessmentIssue {
                collection: Some(collection),
                portable_id: row["portable_id"].as_str().map(str::to_owned),
                field: Some(field.to_owned()),
                reason: AssessmentReason::MissingDependency,
                detail: format!(
                    "The referenced {} portable ID {id} is absent from the source plan.",
                    target.name()
                ),
            });
        }
    };
    for (collection, field, target) in [
        (
            SourceCollection::Medications,
            "location_portable_id",
            SourceCollection::Locations,
        ),
        (
            SourceCollection::DosageOptions,
            "medication_portable_id",
            SourceCollection::Medications,
        ),
        (
            SourceCollection::Schedules,
            "person_portable_id",
            SourceCollection::People,
        ),
        (
            SourceCollection::Schedules,
            "medication_portable_id",
            SourceCollection::Medications,
        ),
        (
            SourceCollection::Schedules,
            "source_dosage_option_portable_id",
            SourceCollection::DosageOptions,
        ),
        (
            SourceCollection::PersonMedications,
            "person_portable_id",
            SourceCollection::People,
        ),
        (
            SourceCollection::PersonMedications,
            "medication_portable_id",
            SourceCollection::Medications,
        ),
        (
            SourceCollection::PersonMedications,
            "source_dosage_option_portable_id",
            SourceCollection::DosageOptions,
        ),
        (
            SourceCollection::MedicationTakes,
            "taken_from_medication_portable_id",
            SourceCollection::Medications,
        ),
        (
            SourceCollection::MedicationTakes,
            "taken_from_location_portable_id",
            SourceCollection::Locations,
        ),
        (
            SourceCollection::NotificationPreferences,
            "person_portable_id",
            SourceCollection::People,
        ),
        (
            SourceCollection::DoseOccurrences,
            "medication_take_portable_id",
            SourceCollection::MedicationTakes,
        ),
        (
            SourceCollection::HealthEvents,
            "person_portable_id",
            SourceCollection::People,
        ),
    ] {
        for row in plan.rows(collection.name()) {
            if let Some(id) = row[field].as_str().filter(|id| !id.is_empty()) {
                check(collection, row, field, target, id);
            }
        }
    }
    for collection in [
        SourceCollection::MedicationTakes,
        SourceCollection::MedicationPausePeriods,
        SourceCollection::DoseOccurrences,
    ] {
        for row in plan.rows(collection.name()) {
            let target = match row["source_type"].as_str() {
                Some("schedule") => SourceCollection::Schedules,
                Some("person_medication") => SourceCollection::PersonMedications,
                _ => continue,
            };
            if let Some(id) = row["source_portable_id"]
                .as_str()
                .filter(|id| !id.is_empty())
            {
                check(collection, row, "source_portable_id", target, id);
            }
        }
    }
    for row in plan.rows("health_events") {
        if let Some(ids) = row["medication_portable_ids"].as_array() {
            for id in ids.iter().filter_map(serde_json::Value::as_str) {
                check(
                    SourceCollection::HealthEvents,
                    row,
                    "medication_portable_ids",
                    SourceCollection::Medications,
                    id,
                );
            }
        }
    }
    issues
}

fn finish_assessment(
    mut rows: Vec<PlannedRow>,
    issues: Vec<AssessmentIssue>,
    aliases: Vec<ResolvedAlias>,
    expectation: DestinationExpectation,
) -> ImportAssessment {
    reject_affected_rows(&mut rows, &issues);
    let (source_compatible, has_blockers) = assessment_status(&issues);
    ImportAssessment {
        rows,
        issues,
        aliases,
        expectation,
        source_compatible,
        has_blockers,
    }
}

fn assessment_status(issues: &[AssessmentIssue]) -> (bool, bool) {
    let source_compatible = !issues.iter().any(|issue| {
        matches!(
            issue.reason,
            AssessmentReason::InvalidSource
                | AssessmentReason::MissingDependency
                | AssessmentReason::IncompatiblePerson
        )
    });
    (source_compatible, !issues.is_empty())
}

fn reject_affected_rows(rows: &mut [PlannedRow], issues: &[AssessmentIssue]) {
    for issue in issues {
        if let (Some(collection), Some(portable_id)) = (issue.collection, &issue.portable_id)
            && let Some(row) = rows
                .iter_mut()
                .find(|row| row.collection == collection && row.portable_id == *portable_id)
        {
            row.action = PlannedAction::Reject;
        }
    }
}

pub(crate) async fn capture_expectation(
    tenant: &TenantTransaction,
    aliases: &[ResolvedAlias],
    plan: &ImportPlan,
    lock: ExpectationLock,
    today: NaiveDate,
) -> Result<DestinationExpectation, OperationError> {
    let authority = super::PreparedImportAuthority::prepare(tenant, today).await?;
    capture_expectation_prepared(&authority, aliases, plan, lock, today).await
}

pub(crate) async fn capture_expectation_prepared(
    authority: &super::PreparedImportAuthority<'_>,
    aliases: &[ResolvedAlias],
    plan: &ImportPlan,
    lock: ExpectationLock,
    today: NaiveDate,
) -> Result<DestinationExpectation, OperationError> {
    authority.check_date(today)?;
    let mut selected: Vec<_> = aliases
        .iter()
        .map(|alias| AliasExpectation {
            source_person_portable_id: alias.source_person_portable_id.clone(),
            destination_person_id: alias.destination_person_id,
            person: None,
            active_membership: None,
        })
        .collect();
    selected.sort_by(|a, b| {
        a.source_person_portable_id
            .cmp(&b.source_person_portable_id)
    });
    let mut slots = Vec::new();
    for collection in SourceCollection::ALL {
        for row in plan.rows(collection.name()) {
            let portable_id = row["portable_id"]
                .as_str()
                .ok_or_else(|| super::import_invalid("portable ID is required"))?;
            let name = collection
                .conflict_field()
                .and_then(|field| row[field].as_str())
                .filter(|value| !value.is_empty())
                .map(|value| NameExpectation {
                    key: value.to_lowercase(),
                    matching_portable_ids: Vec::new(),
                });
            slots.push(SlotExpectation {
                collection,
                portable_id: portable_id.to_owned(),
                occupied: false,
                name,
            });
        }
    }
    slots.sort_by(|a, b| (a.collection, &a.portable_id).cmp(&(b.collection, &b.portable_id)));
    capture(authority, selected, slots, lock).await
}

pub(crate) async fn check_expectation(
    tenant: &TenantTransaction,
    expected: &DestinationExpectation,
    lock: ExpectationLock,
    today: NaiveDate,
) -> Result<bool, OperationError> {
    let authority = super::PreparedImportAuthority::prepare(tenant, today).await?;
    check_expectation_prepared(&authority, expected, lock, today).await
}

pub(crate) async fn check_expectation_prepared(
    authority: &super::PreparedImportAuthority<'_>,
    expected: &DestinationExpectation,
    lock: ExpectationLock,
    today: NaiveDate,
) -> Result<bool, OperationError> {
    authority.check_date(today)?;
    let tenant = authority.tenant();
    if expected.household_id.get() != tenant.scope().household_id
        || expected.actor_id.get() != tenant.scope().actor.account_id
    {
        return Ok(false);
    }
    let current = capture(
        authority,
        expected.aliases.clone(),
        expected.slots.clone(),
        lock,
    )
    .await?;
    Ok(current == *expected)
}

async fn capture(
    authority: &super::PreparedImportAuthority<'_>,
    mut aliases: Vec<AliasExpectation>,
    mut slots: Vec<SlotExpectation>,
    lock: ExpectationLock,
) -> Result<DestinationExpectation, OperationError> {
    let tenant = authority.tenant();
    access::recheck(tenant).await?;
    if lock == ExpectationLock::Confirmation {
        lock_destination(tenant, &aliases, &slots).await?;
    }
    // Recheck after lock waits, before reading the state that will be compared.
    access::recheck(tenant).await?;
    let actor_membership = access::verify_membership(tenant.transaction(), tenant.scope()).await?;
    let index = ExistingIndex::load(tenant.transaction(), tenant.scope().household_id).await?;
    for alias in &mut aliases {
        alias.person = person_queries::household_people(tenant.scope())
            .filter(person::Column::Id.eq(alias.destination_person_id.get()))
            .one(tenant.transaction())
            .await?;
        alias.active_membership = membership::Entity::find()
            .filter(membership::Column::HouseholdId.eq(tenant.scope().household_id))
            .filter(membership::Column::PersonId.eq(alias.destination_person_id.get()))
            .filter(membership::Column::Status.eq("active"))
            .filter(membership::Column::RevokedAt.is_null())
            .one(tenant.transaction())
            .await?;
    }
    for slot in &mut slots {
        slot.occupied = index
            .get(slot.collection.name(), &slot.portable_id)
            .is_some();
        if let Some(name) = &mut slot.name {
            name.matching_portable_ids = matching_names(tenant, slot.collection, &name.key).await?;
        }
    }
    access::recheck(tenant).await?;
    Ok(DestinationExpectation {
        household_id: HouseholdId::from_database(tenant.scope().household_id),
        actor_id: AccountId::from_database(tenant.scope().actor.account_id),
        actor_membership,
        aliases,
        slots,
    })
}

async fn matching_names(
    tenant: &TenantTransaction,
    collection: SourceCollection,
    key: &str,
) -> Result<Vec<String>, OperationError> {
    use sea_orm::sea_query::{Expr, ExprTrait, Func};
    let mut ids = match collection {
        SourceCollection::People => person_queries::household_people(tenant.scope())
            .filter(Func::lower(Expr::col(person::Column::Email)).eq(key))
            .all(tenant.transaction())
            .await?
            .into_iter()
            .map(|row| row.portable_id)
            .collect::<Vec<_>>(),
        SourceCollection::Locations => location::Entity::find()
            .filter(location::Column::HouseholdId.eq(tenant.scope().household_id))
            .filter(Func::lower(Expr::col(location::Column::Name)).eq(key))
            .all(tenant.transaction())
            .await?
            .into_iter()
            .map(|row| row.portable_id)
            .collect(),
        SourceCollection::Medications => medication::Entity::find()
            .filter(medication::Column::HouseholdId.eq(tenant.scope().household_id))
            .filter(Func::lower(Expr::col(medication::Column::Name)).eq(key))
            .all(tenant.transaction())
            .await?
            .into_iter()
            .map(|row| row.portable_id)
            .collect(),
        _ => return Err(super::import_invalid("unsupported name expectation")),
    };
    ids.sort();
    Ok(ids)
}

async fn lock_destination(
    tenant: &TenantTransaction,
    aliases: &[AliasExpectation],
    slots: &[SlotExpectation],
) -> Result<(), OperationError> {
    let has_names = |collection| {
        slots
            .iter()
            .any(|slot| slot.collection == collection && slot.name.is_some())
    };
    let mut membership_people: Vec<i64> = aliases
        .iter()
        .map(|alias| alias.destination_person_id.get())
        .collect();
    membership_people.push(tenant.membership().person_id);
    let mut people_ids = membership_people.clone();
    if has_names(SourceCollection::People) {
        people_ids.extend(
            person_queries::household_people(tenant.scope())
                .all(tenant.transaction())
                .await?
                .into_iter()
                .map(|row| row.id),
        );
    }
    person_queries::household_people(tenant.scope())
        .filter(person::Column::Id.is_in(people_ids))
        .order_by_asc(person::Column::Id)
        .lock_exclusive()
        .all(tenant.transaction())
        .await?;
    membership::Entity::find()
        .filter(membership::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(membership::Column::PersonId.is_in(membership_people))
        .order_by_asc(membership::Column::Id)
        .lock_exclusive()
        .all(tenant.transaction())
        .await?;
    if has_names(SourceCollection::Locations) {
        location::Entity::find()
            .filter(location::Column::HouseholdId.eq(tenant.scope().household_id))
            .order_by_asc(location::Column::Id)
            .lock_exclusive()
            .all(tenant.transaction())
            .await?;
    }
    if has_names(SourceCollection::Medications) {
        medication::Entity::find()
            .filter(medication::Column::HouseholdId.eq(tenant.scope().household_id))
            .order_by_asc(medication::Column::Id)
            .lock_exclusive()
            .all(tenant.transaction())
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_dependencies_are_attributed_to_the_source_row() {
        let plan = ImportPlan::parse(json!({"format":"medtracker.portable.v3","records":{
            "schedules":[{"portable_id":"schedule", "person_portable_id":"destination", "medication_portable_id":"missing"}]
        }})).unwrap();
        let aliases = vec![ResolvedAlias {
            source_person_portable_id: "source-person".into(),
            destination_person_id: PersonId::from_database(7),
            destination_person_portable_id: "destination".into(),
            profile_differences: vec![],
        }];
        let issues = missing_dependencies(&plan, &aliases);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].collection, Some(SourceCollection::Schedules));
        assert_eq!(issues[0].portable_id.as_deref(), Some("schedule"));
        assert_eq!(issues[0].field.as_deref(), Some("medication_portable_id"));
        assert_eq!(issues[0].reason, AssessmentReason::MissingDependency);
    }

    #[test]
    fn a_person_conflict_rejects_only_the_identified_person() {
        let mut rows = vec![
            PlannedRow {
                collection: SourceCollection::People,
                portable_id: "one".into(),
                action: PlannedAction::Match,
            },
            PlannedRow {
                collection: SourceCollection::People,
                portable_id: "two".into(),
                action: PlannedAction::Match,
            },
        ];
        let issues = vec![AssessmentIssue {
            collection: Some(SourceCollection::People),
            portable_id: Some("one".into()),
            field: Some("date_of_birth".into()),
            reason: AssessmentReason::IncompatiblePerson,
            detail: "Date of birth differs".into(),
        }];
        reject_affected_rows(&mut rows, &issues);
        assert_eq!(rows[0].action, PlannedAction::Reject);
        assert_eq!(rows[1].action, PlannedAction::Match);
    }

    #[test]
    fn destination_conflict_blocks_without_claiming_source_is_invalid() {
        let mut issue = AssessmentIssue {
            collection: Some(SourceCollection::Medications),
            portable_id: Some("m".into()),
            field: Some("portable_id".into()),
            reason: AssessmentReason::OccupiedSourceId,
            detail: "Occupied".into(),
        };
        assert_eq!(
            assessment_status(std::slice::from_ref(&issue)),
            (true, true)
        );
        issue.reason = AssessmentReason::MissingDependency;
        assert_eq!(assessment_status(&[issue]), (false, true));
        assert_eq!(assessment_status(&[]), (true, false));
    }

    #[test]
    fn slot_comparison_detects_occupation_and_name_conflicts() {
        let original = SlotExpectation {
            collection: SourceCollection::Medications,
            portable_id: "source".into(),
            occupied: false,
            name: Some(NameExpectation {
                key: "medicine".into(),
                matching_portable_ids: vec![],
            }),
        };
        let mut changed = original.clone();
        changed.occupied = true;
        assert_ne!(original, changed);
        changed.occupied = false;
        changed
            .name
            .as_mut()
            .unwrap()
            .matching_portable_ids
            .push("existing".into());
        assert_ne!(original, changed);
    }
}
