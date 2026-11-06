use std::collections::{HashMap, HashSet};

use chrono::NaiveDate;
use sea_orm::{ColumnTrait, Condition, QueryFilter};
use serde_json::Value;

use super::ImportPlan;
use crate::models::{
    access::{TenantTransaction, person_queries},
    entities::person,
    errors::OperationError,
    ids::PersonId,
};

pub(crate) struct AliasSelection {
    pub(crate) source_person_portable_id: String,
    pub(crate) destination_person_id: PersonId,
}

#[derive(PartialEq, Eq)]
pub(crate) enum ProfileDifference {
    Name {
        source: String,
        destination: String,
    },
    Email {
        source: Option<String>,
        destination: Option<String>,
    },
}

pub(crate) struct ResolvedAlias {
    pub(crate) source_person_portable_id: String,
    pub(crate) destination_person_id: PersonId,
    pub(crate) destination_person_portable_id: String,
    pub(crate) profile_differences: Vec<ProfileDifference>,
}

pub(crate) struct NormalizedImport {
    pub(crate) source_payload: Value,
    pub(crate) aliases: Vec<ResolvedAlias>,
    pub(crate) plan: ImportPlan,
}

#[derive(Debug)]
pub(crate) enum AliasError {
    InvalidPayload,
    InvalidSelection,
    IncompatiblePerson,
    Database(OperationError),
}

struct DestinationPerson {
    id: PersonId,
    portable_id: String,
    name: String,
    email: Option<String>,
    date_of_birth: Option<NaiveDate>,
    person_type: i32,
    has_capacity: bool,
}

pub(crate) async fn normalize_aliases(
    tenant: &TenantTransaction,
    source_payload: Value,
    selections: &[AliasSelection],
    new_dependents: &[String],
) -> Result<NormalizedImport, AliasError> {
    let ids = selections
        .iter()
        .map(|selection| selection.destination_person_id.get());
    let people = person_queries::household_people(tenant.scope())
        .filter(
            Condition::any()
                .add(person::Column::Id.is_in(ids))
                .add(person::Column::PortableId.is_in(new_dependents.iter().cloned())),
        )
        .all(tenant.transaction())
        .await
        .map_err(|error| AliasError::Database(error.into()))?;
    let destinations: Vec<DestinationPerson> = people
        .into_iter()
        .map(|person| DestinationPerson {
            id: PersonId::from_database(person.id),
            portable_id: person.portable_id,
            name: person.name,
            email: person.email,
            date_of_birth: person.date_of_birth,
            person_type: person.person_type,
            has_capacity: person.has_capacity,
        })
        .collect();
    normalize_resolved(source_payload, selections, new_dependents, &destinations)
}

fn normalize_resolved(
    source_payload: Value,
    selections: &[AliasSelection],
    new_dependents: &[String],
    destinations: &[DestinationPerson],
) -> Result<NormalizedImport, AliasError> {
    let source_plan =
        ImportPlan::parse(source_payload.clone()).map_err(|_| AliasError::InvalidPayload)?;
    let mut selected_sources = HashSet::new();
    let mut selected_destinations = HashSet::new();
    let mut aliases = Vec::with_capacity(selections.len());
    let source_people: HashMap<&str, &Value> = source_plan
        .rows("people")
        .iter()
        .filter_map(|row| row["portable_id"].as_str().map(|id| (id, row)))
        .collect();
    for selection in selections {
        if !selected_sources.insert(selection.source_person_portable_id.as_str())
            || !selected_destinations.insert(selection.destination_person_id.get())
        {
            return Err(AliasError::InvalidSelection);
        }
        let source = source_people
            .get(selection.source_person_portable_id.as_str())
            .ok_or(AliasError::InvalidSelection)?;
        let destination = destinations
            .iter()
            .find(|person| person.id == selection.destination_person_id)
            .ok_or(AliasError::InvalidSelection)?;
        let birth_value = source
            .get("date_of_birth")
            .ok_or(AliasError::IncompatiblePerson)?;
        let birth = if birth_value.is_null() {
            None
        } else {
            Some(
                NaiveDate::parse_from_str(
                    birth_value.as_str().ok_or(AliasError::IncompatiblePerson)?,
                    "%Y-%m-%d",
                )
                .map_err(|_| AliasError::IncompatiblePerson)?,
            )
        };
        let kind = match source["person_type"].as_str() {
            Some("adult") => 0,
            Some("minor") => 1,
            Some("dependent_adult") => 2,
            _ => return Err(AliasError::IncompatiblePerson),
        };
        let capacity = source["has_capacity"]
            .as_bool()
            .ok_or(AliasError::IncompatiblePerson)?;
        if birth != destination.date_of_birth
            || kind != destination.person_type
            || capacity != destination.has_capacity
        {
            return Err(AliasError::IncompatiblePerson);
        }
        let name = source["name"].as_str().ok_or(AliasError::InvalidPayload)?;
        let email = match source.get("email") {
            None | Some(Value::Null) => None,
            Some(Value::String(email)) => Some(email.clone()),
            Some(_) => return Err(AliasError::InvalidPayload),
        };
        let mut differences = Vec::new();
        if name != destination.name {
            differences.push(ProfileDifference::Name {
                source: name.to_owned(),
                destination: destination.name.clone(),
            });
        }
        if email != destination.email {
            differences.push(ProfileDifference::Email {
                source: email,
                destination: destination.email.clone(),
            });
        }
        aliases.push(ResolvedAlias {
            source_person_portable_id: selection.source_person_portable_id.clone(),
            destination_person_id: destination.id,
            destination_person_portable_id: destination.portable_id.clone(),
            profile_differences: differences,
        });
    }
    for id in new_dependents {
        if !selected_sources.insert(id.as_str())
            || !source_people.contains_key(id.as_str())
            || destinations.iter().any(|person| person.portable_id == *id)
        {
            return Err(AliasError::InvalidSelection);
        }
    }
    if selected_sources.len() != source_people.len() {
        return Err(AliasError::InvalidSelection);
    }
    let mut normalized = source_payload.clone();
    let records = normalized
        .get_mut("records")
        .and_then(Value::as_object_mut)
        .ok_or(AliasError::InvalidPayload)?;
    if let Some(people) = records.get_mut("people").and_then(Value::as_array_mut) {
        people.retain(|row| {
            !selections.iter().any(|selection| {
                row["portable_id"].as_str() == Some(selection.source_person_portable_id.as_str())
            })
        });
    }
    for kind in [
        "schedules",
        "person_medications",
        "notification_preferences",
        "health_events",
    ] {
        if let Some(rows) = records.get_mut(kind).and_then(Value::as_array_mut) {
            for row in rows {
                if let Some(alias) = aliases.iter().find(|alias| {
                    row["person_portable_id"].as_str()
                        == Some(alias.source_person_portable_id.as_str())
                }) {
                    row["person_portable_id"] =
                        Value::String(alias.destination_person_portable_id.clone());
                }
            }
        }
    }
    let mut plan = ImportPlan::parse(normalized).map_err(|_| AliasError::InvalidPayload)?;
    let mut errors = source_plan.errors.clone();
    for (index, row) in source_plan.rows("people").iter().enumerate() {
        if selections.iter().any(|selection| {
            row["portable_id"].as_str() == Some(selection.source_person_portable_id.as_str())
        }) {
            errors.extend(super::validate_row("people", index, row));
        }
    }
    plan.errors = errors;
    Ok(NormalizedImport {
        source_payload,
        aliases,
        plan,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use serde_json::json;

    fn payload() -> Value {
        json!({"format":"medtracker.portable.v3","records":{
            "people":[
                {"portable_id":"source-adult","name":"Source name","email":"source@example.test","date_of_birth":"1980-01-02","person_type":"adult","has_capacity":true},
                {"portable_id":"new-child","name":"Child","date_of_birth":"2018-01-02","person_type":"minor","has_capacity":false}
            ],
            "schedules":[{"portable_id":"s","person_portable_id":"source-adult","medication_portable_id":"m"}],
            "person_medications":[{"portable_id":"a","person_portable_id":"source-adult","medication_portable_id":"m","position":0}],
            "notification_preferences":[{"portable_id":"n","person_portable_id":"source-adult"}],
            "health_events":[{"portable_id":"h","person_portable_id":"source-adult","event_kind":"illness","title":"Event","started_on":"2020-01-01"}],
            "medication_pause_periods":[{"portable_id":"p","source_type":"schedule","source_portable_id":"s","reason":"other","recorded_by_person_portable_id":"source-adult","resumed_by_person_portable_id":"source-adult"}]
        }})
    }

    fn destination() -> DestinationPerson {
        DestinationPerson {
            id: PersonId::from_database(7),
            portable_id: "destination-adult".into(),
            name: "Destination name".into(),
            email: Some("destination@example.test".into()),
            date_of_birth: NaiveDate::from_ymd_opt(1980, 1, 2),
            person_type: 0,
            has_capacity: true,
        }
    }

    fn selection(source: &str, id: i64) -> AliasSelection {
        AliasSelection {
            source_person_portable_id: source.into(),
            destination_person_id: PersonId::from_database(id),
        }
    }

    #[test]
    fn normalizes_only_clinical_references_and_preserves_source_and_profiles() {
        let original = payload();
        let result = normalize_resolved(
            original.clone(),
            &[selection("source-adult", 7)],
            &["new-child".into()],
            &[destination()],
        )
        .unwrap();
        assert_eq!(result.source_payload, original);
        assert_eq!(result.plan.rows("people").len(), 1);
        assert_eq!(result.plan.rows("people")[0]["portable_id"], "new-child");
        for kind in [
            "schedules",
            "person_medications",
            "notification_preferences",
            "health_events",
        ] {
            assert_eq!(
                result.plan.rows(kind)[0]["person_portable_id"],
                "destination-adult"
            );
        }
        let pause = &result.plan.rows("medication_pause_periods")[0];
        assert_eq!(pause["recorded_by_person_portable_id"], "source-adult");
        assert_eq!(pause["resumed_by_person_portable_id"], "source-adult");
        let alias = &result.aliases[0];
        assert_eq!(alias.source_person_portable_id, "source-adult");
        assert_eq!(alias.destination_person_id, PersonId::from_database(7));
        assert_eq!(alias.destination_person_portable_id, "destination-adult");
        assert_eq!(alias.profile_differences.len(), 2);
        assert!(
            alias
                .profile_differences
                .contains(&ProfileDifference::Name {
                    source: "Source name".into(),
                    destination: "Destination name".into()
                })
        );
        assert!(
            alias
                .profile_differences
                .contains(&ProfileDifference::Email {
                    source: Some("source@example.test".into()),
                    destination: Some("destination@example.test".into())
                })
        );
    }

    #[test]
    fn rejects_incomplete_duplicate_and_unknown_selections() {
        let dest = destination();
        for (aliases, new_people) in [
            (vec![selection("source-adult", 7)], vec![]),
            (
                vec![selection("source-adult", 7), selection("source-adult", 7)],
                vec!["new-child".into()],
            ),
            (
                vec![selection("source-adult", 7), selection("new-child", 7)],
                vec![],
            ),
            (vec![selection("missing", 7)], vec!["new-child".into()]),
            (vec![selection("source-adult", 8)], vec!["new-child".into()]),
            (
                vec![selection("source-adult", 7)],
                vec!["new-child".into(), "new-child".into()],
            ),
            (
                vec![selection("source-adult", 7)],
                vec!["source-adult".into(), "new-child".into()],
            ),
        ] {
            assert!(
                normalize_resolved(
                    payload(),
                    &aliases,
                    &new_people,
                    std::slice::from_ref(&dest)
                )
                .is_err()
            );
        }
    }

    #[test]
    fn rejects_incompatible_identity_and_new_person_collisions() {
        for field in ["date_of_birth", "person_type", "has_capacity"] {
            let mut input = payload();
            input["records"]["people"][0][field] = match field {
                "date_of_birth" => json!("1981-01-02"),
                "person_type" => json!("minor"),
                _ => json!(false),
            };
            assert!(
                normalize_resolved(
                    input,
                    &[selection("source-adult", 7)],
                    &["new-child".into()],
                    &[destination()]
                )
                .is_err()
            );
        }
        let mut collision = destination();
        collision.id = PersonId::from_database(8);
        collision.portable_id = "new-child".into();
        assert!(
            normalize_resolved(
                payload(),
                &[selection("source-adult", 7)],
                &["new-child".into()],
                &[destination(), collision]
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_missing_compatibility_fields_and_preserves_mapped_validation_errors() {
        let mut missing = payload();
        missing["records"]["people"][0]
            .as_object_mut()
            .unwrap()
            .remove("date_of_birth");
        assert!(matches!(
            normalize_resolved(
                missing,
                &[selection("source-adult", 7)],
                &["new-child".into()],
                &[destination()]
            ),
            Err(AliasError::IncompatiblePerson)
        ));

        let mut invalid = payload();
        invalid["records"]["people"][0]["name"] = json!("   ");
        let result = normalize_resolved(
            invalid,
            &[selection("source-adult", 7)],
            &["new-child".into()],
            &[destination()],
        )
        .unwrap();
        assert!(!result.plan.errors.is_empty());
    }
}
