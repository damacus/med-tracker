use super::{ExistingIndex, ImportPlan, ResolvedAlias};
use crate::models::{
    access::{self, TenantTransaction, person_queries},
    entities::{household, location, medication, membership, person},
    errors::OperationError,
    ids::{AccountId, HouseholdId, PersonId},
};
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

pub(crate) async fn capture_expectation(
    tenant: &TenantTransaction,
    aliases: &[ResolvedAlias],
    plan: &ImportPlan,
    lock: ExpectationLock,
) -> Result<DestinationExpectation, OperationError> {
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
    capture(tenant, selected, slots, lock).await
}

pub(crate) async fn check_expectation(
    tenant: &TenantTransaction,
    expected: &DestinationExpectation,
    lock: ExpectationLock,
) -> Result<bool, OperationError> {
    if expected.household_id.get() != tenant.scope().household_id
        || expected.actor_id.get() != tenant.scope().actor.account_id
    {
        return Ok(false);
    }
    let current = capture(
        tenant,
        expected.aliases.clone(),
        expected.slots.clone(),
        lock,
    )
    .await?;
    Ok(current == *expected)
}

async fn capture(
    tenant: &TenantTransaction,
    mut aliases: Vec<AliasExpectation>,
    mut slots: Vec<SlotExpectation>,
    lock: ExpectationLock,
) -> Result<DestinationExpectation, OperationError> {
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
    household::Entity::find_by_id(tenant.scope().household_id)
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
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
