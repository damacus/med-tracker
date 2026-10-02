use super::*;

pub(super) async fn authorized(
    db: &DatabaseTransaction,
    context: &AuthContext,
    plan: &ImportPlan,
    index: &ExistingIndex,
) -> Result<bool, ApiError> {
    if household_manager(context) {
        return Ok(true);
    }
    if !plan.rows("locations").is_empty() {
        return Ok(false);
    }
    let grants = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(context.membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(grant::Column::RevokedAt.is_null())
        .all(db)
        .await
        .map_err(database_error)?;
    let now = Utc::now().naive_utc();
    let manageable_ids: HashSet<i64> = grants
        .into_iter()
        .filter(|grant| {
            grant.access_level == "manage" && grant.expires_at.is_none_or(|expires| expires > now)
        })
        .map(|grant| grant.person_id)
        .collect();
    let mut referenced_people = HashSet::new();
    for kind in [
        "people",
        "schedules",
        "person_medications",
        "notification_preferences",
        "health_events",
    ] {
        for row in plan.rows(kind) {
            let field = if kind == "people" {
                "portable_id"
            } else {
                "person_portable_id"
            };
            if let Some(person) = row[field].as_str() {
                referenced_people.insert(person.to_owned());
            }
            if let Some(person) = row["portable_id"]
                .as_str()
                .and_then(|id| index.owner(kind, id))
            {
                referenced_people.insert(person.to_owned());
            }
        }
    }
    for kind in [
        "medication_takes",
        "medication_pause_periods",
        "dose_occurrences",
    ] {
        for row in plan.rows(kind) {
            if let Some(person) = row["portable_id"]
                .as_str()
                .and_then(|id| index.owner(kind, id))
            {
                referenced_people.insert(person.to_owned());
            }
            let source_kind = match row["source_type"].as_str() {
                Some("schedule") => "schedules",
                Some("person_medication") => "person_medications",
                _ => return Ok(false),
            };
            let Some(source_id) = row["source_portable_id"].as_str() else {
                return Ok(false);
            };
            if let Some(person) = index.owner(source_kind, source_id) {
                referenced_people.insert(person.to_owned());
            }
            if let Some(person) = plan
                .rows(source_kind)
                .iter()
                .find(|source| source["portable_id"] == source_id)
                .and_then(|source| source["person_portable_id"].as_str())
            {
                referenced_people.insert(person.to_owned());
            }
        }
    }
    if referenced_people.is_empty()
        || referenced_people.iter().any(|portable_id| {
            index
                .get("people", portable_id)
                .is_none_or(|id| !manageable_ids.contains(&id))
        })
    {
        return Ok(false);
    }
    let incoming_linked_medications: HashSet<String> = ["schedules", "person_medications"]
        .iter()
        .flat_map(|kind| {
            plan.rows(kind).iter().filter_map(|row| {
                let person_id = row["person_portable_id"].as_str()?;
                if !referenced_people.contains(person_id) {
                    return None;
                }
                row["medication_portable_id"].as_str().map(str::to_owned)
            })
        })
        .collect();
    let mut existing_authorized_medications = HashSet::new();
    for kind in ["schedules", "person_medications"] {
        if let (Some(owners), Some(medications)) =
            (index.owners.get(kind), index.source_medications.get(kind))
        {
            for (source_id, owner) in owners {
                if index
                    .get("people", owner)
                    .is_some_and(|id| manageable_ids.contains(&id))
                {
                    if let Some(medication) = medications.get(source_id) {
                        existing_authorized_medications.insert(medication.clone());
                    }
                }
            }
        }
    }
    let existing_medications: HashSet<String> = index
        .ids
        .get("medications")
        .map(|ids| ids.keys().cloned().collect())
        .unwrap_or_default();
    let planned_new_medications: HashSet<String> = plan
        .rows("medications")
        .iter()
        .filter_map(|row| row["portable_id"].as_str())
        .filter(|id| {
            !existing_medications.contains(*id) && incoming_linked_medications.contains(*id)
        })
        .map(str::to_owned)
        .collect();
    let target_allowed = |id: &str| {
        delegated_medication_target_allowed(
            id,
            &existing_authorized_medications,
            &planned_new_medications,
            &existing_medications,
        )
    };
    if ["schedules", "person_medications"].iter().any(|kind| {
        plan.rows(kind).iter().any(|row| {
            row["medication_portable_id"]
                .as_str()
                .is_none_or(|id| !target_allowed(id))
        })
    }) || plan.rows("medications").iter().any(|row| {
        row["portable_id"]
            .as_str()
            .is_none_or(|id| !target_allowed(id))
    }) || plan.rows("dosage_options").iter().any(|row| {
        let Some(target) = row["medication_portable_id"].as_str() else {
            return true;
        };
        let existing_parent = row["portable_id"]
            .as_str()
            .and_then(|id| index.dosage_medications.get(id).map(String::as_str));
        !delegated_dosage_write_allowed(
            existing_parent,
            target,
            &existing_authorized_medications,
            &planned_new_medications,
            &existing_medications,
        )
    }) {
        return Ok(false);
    }
    Ok(true)
}

fn delegated_medication_target_allowed(
    target: &str,
    existing_authorized: &HashSet<String>,
    planned_new: &HashSet<String>,
    existing: &HashSet<String>,
) -> bool {
    if existing.contains(target) {
        existing_authorized.contains(target)
    } else {
        planned_new.contains(target)
    }
}

fn delegated_dosage_write_allowed(
    current_parent: Option<&str>,
    target: &str,
    existing_authorized: &HashSet<String>,
    planned_new: &HashSet<String>,
    existing: &HashSet<String>,
) -> bool {
    current_parent.is_none_or(|parent| existing_authorized.contains(parent))
        && delegated_medication_target_allowed(target, existing_authorized, planned_new, existing)
}

#[cfg(test)]
mod tests {
    use super::{delegated_dosage_write_allowed, delegated_medication_target_allowed};
    use std::collections::HashSet;

    #[test]
    fn delegated_import_cannot_bootstrap_existing_medication_or_reparent_dosage() {
        let authorized: HashSet<String> = ["managed-med".to_owned()].into();
        let planned_new: HashSet<String> = ["new-med".to_owned()].into();
        let existing: HashSet<String> = ["managed-med".to_owned(), "other-med".to_owned()].into();

        assert!(!delegated_medication_target_allowed(
            "other-med",
            &authorized,
            &planned_new,
            &existing
        ));
        assert!(delegated_medication_target_allowed(
            "managed-med",
            &authorized,
            &planned_new,
            &existing
        ));
        assert!(delegated_medication_target_allowed(
            "new-med",
            &authorized,
            &planned_new,
            &existing
        ));
        assert!(!delegated_medication_target_allowed(
            "unplanned-new-med",
            &authorized,
            &planned_new,
            &existing
        ));
        assert!(!delegated_medication_target_allowed(
            "managed-med",
            &HashSet::new(),
            &planned_new,
            &existing
        ));
        assert!(!delegated_dosage_write_allowed(
            Some("other-med"),
            "managed-med",
            &authorized,
            &planned_new,
            &existing
        ));
        assert!(delegated_dosage_write_allowed(
            Some("managed-med"),
            "managed-med",
            &authorized,
            &planned_new,
            &existing
        ));
        assert!(!delegated_dosage_write_allowed(
            None,
            "other-med",
            &authorized,
            &planned_new,
            &existing
        ));
        assert!(delegated_dosage_write_allowed(
            None,
            "new-med",
            &authorized,
            &planned_new,
            &existing
        ));
    }
}
