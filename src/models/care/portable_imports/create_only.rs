use super::*;
use crate::models::entities::{
    dosage, dose_occurrence, health_event, health_event_medication, location, medication,
    medication_take, notification_preference, pause_period, person, person_medication, schedule,
};
use sea_orm::{ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, Iterable, QueryFilter};

pub(crate) enum CreatedRecord {
    Person(Box<person::Model>),
    Location(Box<location::Model>),
    Medication(Box<medication::Model>),
    DosageOption(Box<dosage::Model>),
    Schedule(Box<schedule::Model>),
    Assignment(Box<person_medication::Model>),
    Pause(Box<pause_period::Model>),
    Dose(Box<medication_take::Model>),
    NotificationPreference(Box<notification_preference::Model>),
    Occurrence(Box<dose_occurrence::Model>),
    HealthEvent(Box<health_event::Model>),
}

enum ExpectedRecord {
    Person(i64, Box<person::ActiveModel>),
    Location(i64, Box<location::ActiveModel>),
    Medication(i64, Box<medication::ActiveModel>),
    DosageOption(i64, Box<dosage::ActiveModel>),
    Schedule(i64, Box<schedule::ActiveModel>),
    Assignment(i64, Box<person_medication::ActiveModel>),
    Pause(i64, Box<pause_period::ActiveModel>),
    Dose(i64, Box<medication_take::ActiveModel>),
    NotificationPreference(i64, Box<notification_preference::ActiveModel>),
    Occurrence(i64, Box<dose_occurrence::ActiveModel>),
    HealthEvent(i64, Box<health_event::ActiveModel>),
}

fn same_fields<A: ActiveModelTrait>(expected: &A, actual: &A) -> bool {
    <<<A as ActiveModelTrait>::Entity as EntityTrait>::Column as Iterable>::iter().all(|column| {
        match expected.get(column) {
            ActiveValue::Set(value) | ActiveValue::Unchanged(value) => {
                matches!(actual.get(column), ActiveValue::Set(current) | ActiveValue::Unchanged(current) if current == value)
            }
            ActiveValue::NotSet => true,
        }
    })
}

fn same_health_edges(mut expected: Vec<(i64, String)>, mut actual: Vec<(i64, String)>) -> bool {
    expected.sort();
    actual.sort();
    expected == actual
}

fn document_fields(kind: &str, mut fields: Map<String, Value>) -> Result<Value, OperationError> {
    for key in [
        "created_at",
        "retired_at",
        "taken_at",
        "started_at",
        "ended_at",
        "scheduled_at",
        "resolved_at",
    ] {
        if let Some(value) = fields.get_mut(key)
            && !value.is_null()
        {
            let time = value
                .as_str()
                .and_then(|text| DateTime::parse_from_rfc3339(text).ok())
                .ok_or_else(|| import_invalid("Invalid portable timestamp"))?;
            *value = json!(time.naive_utc());
        }
    }
    for key in [
        "amount",
        "dose_amount",
        "current_supply",
        "reorder_threshold",
        "default_min_hours_between_doses",
    ] {
        if let Some(value) = fields.get_mut(key)
            && !value.is_null()
        {
            let number = decimal(value).ok_or_else(|| import_invalid("Invalid portable amount"))?;
            if kind == "medications" && key == "dose_amount" {
                let double: f64 = number
                    .to_string()
                    .parse()
                    .map_err(|_| import_invalid("Invalid medication dose"))?;
                if Decimal::from_f64_retain(double) != Some(number) {
                    return Err(import_invalid(
                        "Medication dose cannot be represented exactly",
                    ));
                }
                *value = json!(double);
            } else {
                *value = Value::String(number.to_string());
            }
        }
    }
    let now = json!(Utc::now().naive_utc());
    fields.entry("created_at").or_insert_with(|| now.clone());
    fields.insert("updated_at".into(), now);
    Ok(Value::Object(fields))
}

fn has_existing_record(plan: &ImportPlan, mut exists: impl FnMut(&str, &str) -> bool) -> bool {
    for kind in BASE_TYPES.iter().chain(V2_TYPES) {
        for row in plan.rows(kind) {
            if row["portable_id"]
                .as_str()
                .is_some_and(|id| exists(kind, id))
            {
                return true;
            }
        }
    }
    false
}

fn ensure_new(plan: &ImportPlan, index: &ExistingIndex) -> Result<(), OperationError> {
    if has_existing_record(plan, |kind, id| index.get(kind, id).is_some()) {
        return Err(import_invalid("Portable record already exists"));
    }
    for kind in [
        "medication_pause_periods",
        "medication_takes",
        "dose_occurrences",
    ] {
        for row in plan.rows(kind) {
            let source_kind = match row["source_type"].as_str() {
                Some("schedule") => "schedules",
                Some("person_medication") => "person_medications",
                _ => return Err(import_invalid("Invalid source type")),
            };
            if !plan
                .rows(source_kind)
                .iter()
                .any(|source| source["portable_id"] == row["source_portable_id"])
            {
                return Err(import_invalid(
                    "Historical source must be created by this import",
                ));
            }
        }
    }
    Ok(())
}

async fn clinical_authority(
    tenant: &TenantTransaction,
    plan: &ImportPlan,
    index: &ExistingIndex,
    declarations: &PreparedDeclarations,
) -> Result<(), OperationError> {
    let mut people = HashSet::new();
    for kind in ["schedules", "person_medications", "health_events"] {
        for row in plan.rows(kind) {
            let portable = row["person_portable_id"]
                .as_str()
                .ok_or_else(|| import_invalid("Clinical person is unavailable"))?;
            if !people.insert(portable) || declarations.contains(portable) {
                continue;
            }
            let id = index
                .get("people", portable)
                .ok_or_else(|| import_invalid("Clinical person is unavailable"))?;
            if !super::super::person_schedules::sharing::clinical_authority(
                tenant.transaction(),
                crate::models::ids::PersonId::from_database(id),
            )
            .await?
            {
                return Err(import_invalid("Current clinical authority is required"));
            }
        }
    }
    Ok(())
}

pub(crate) async fn create_only_records(
    tenant: &TenantTransaction,
    plan: &ImportPlan,
    index: &mut ExistingIndex,
    declarations: &mut PreparedDeclarations,
    provenance: &CredentialProvenance,
    request_id: &str,
    eligibility_zone: chrono_tz::Tz,
) -> Result<Vec<CreatedRecord>, OperationError> {
    if !plan.rows("people").is_empty() {
        super::super::guardian_declarations::lock_creator_account(tenant).await?;
    }
    super::super::sync::lock(tenant).await?;
    let today = Utc::now().with_timezone(&eligibility_zone).date_naive();
    if !declarations.is_fresh(today) {
        return Err(import_invalid(
            "Initial declarations require a fresh preview",
        ));
    }
    *index = ExistingIndex::load(tenant.transaction(), tenant.scope().household_id).await?;
    ensure_new(plan, index)?;
    if !authorized(tenant, plan, index, declarations, today).await? {
        return Err(OperationError::Forbidden);
    }
    let result = preflight(tenant.transaction(), tenant, plan, index).await?;
    if !result.errors.is_empty() || !result.conflicts.is_empty() {
        return Err(import_invalid(
            "Portable import has conflicts or invalid records",
        ));
    }
    clinical_authority(tenant, plan, index, declarations).await?;

    let db = tenant.transaction();
    let mut expected = Vec::new();
    let mut health_edges: Vec<(i64, Vec<(i64, String)>)> = Vec::new();
    for kind in [
        "locations",
        "people",
        "medications",
        "dosage_options",
        "schedules",
        "person_medications",
        "medication_pause_periods",
        "medication_takes",
        "notification_preferences",
        "dose_occurrences",
        "health_events",
    ] {
        let mut rows: Vec<&Value> = plan.rows(kind).iter().collect();
        if kind == "medication_pause_periods" {
            rows.sort_by_key(|row| row["ended_at"].is_null());
        }
        for row in rows {
            let portable_id = text(row, "portable_id").map_err(import_invalid)?;
            let fields = mapped_row(kind, row, index, tenant).map_err(import_invalid)?;
            let document = document_fields(kind, fields)?;
            macro_rules! insert {
                ($entity:ident, $variant:ident) => {{
                    let mut active = $entity::ActiveModel::from_json(document)
                        .map_err(|_| import_invalid("Portable record cannot be decoded"))?;
                    let created = active.clone().insert(db).await?;
                    let id = created.id;
                    active.not_set($entity::Column::UpdatedAt);
                    if kind != "medication_pause_periods" || row["created_at"].is_null() {
                        active.not_set($entity::Column::CreatedAt);
                    }
                    expected.push(ExpectedRecord::$variant(id, Box::new(active)));
                    (id, json!(&created))
                }};
            }
            let (id, after) = match kind {
                "people" => {
                    let mut active = person::ActiveModel::from_json(document)
                        .map_err(|_| import_invalid("Portable person cannot be decoded"))?;
                    let id = declarations
                        .create(tenant, &portable_id, provenance)
                        .await?;
                    let created = person::Entity::find_by_id(id)
                        .one(db)
                        .await?
                        .ok_or(OperationError::Unavailable)?;
                    active.not_set(person::Column::CreatedAt);
                    active.not_set(person::Column::UpdatedAt);
                    expected.push(ExpectedRecord::Person(id, Box::new(active)));
                    (id, json!(&created))
                }
                "locations" => insert!(location, Location),
                "medications" => insert!(medication, Medication),
                "dosage_options" => insert!(dosage, DosageOption),
                "schedules" => insert!(schedule, Schedule),
                "person_medications" => insert!(person_medication, Assignment),
                "medication_pause_periods" => insert!(pause_period, Pause),
                "medication_takes" => insert!(medication_take, Dose),
                "notification_preferences" => {
                    insert!(notification_preference, NotificationPreference)
                }
                "dose_occurrences" => insert!(dose_occurrence, Occurrence),
                "health_events" => insert!(health_event, HealthEvent),
                _ => return Err(OperationError::Unavailable),
            };
            index.put(kind, portable_id.clone(), id);
            if kind == "health_events" {
                let mut edges = Vec::new();
                if let Some(medications) = row["medication_portable_ids"].as_array() {
                    for portable in medications {
                        let medication_id = portable
                            .as_str()
                            .and_then(|portable| index.get("medications", portable))
                            .ok_or_else(|| {
                                import_invalid("Health event medication is unavailable")
                            })?;
                        let medication = medication::Entity::find_by_id(medication_id)
                            .filter(medication::Column::HouseholdId.eq(tenant.scope().household_id))
                            .one(db)
                            .await?
                            .ok_or(OperationError::Unavailable)?;
                        let name = medication
                            .name
                            .or(medication.friendly_name)
                            .unwrap_or_else(|| "Medication".into());
                        let now = Utc::now().naive_utc();
                        health_event_medication::ActiveModel {
                            household_id: Set(tenant.scope().household_id),
                            health_event_id: Set(id),
                            medication_id: Set(Some(medication_id)),
                            medication_name: Set(name.clone()),
                            created_at: Set(now),
                            updated_at: Set(now),
                            ..Default::default()
                        }
                        .insert(db)
                        .await?;
                        edges.push((medication_id, name));
                    }
                }
                health_edges.push((id, edges));
            }
            let record_type = match kind {
                "people" => "Person",
                "locations" => "Location",
                "medications" => "Medication",
                "dosage_options" => "MedicationDosageOption",
                "schedules" => "Schedule",
                "person_medications" => "PersonMedication",
                "medication_pause_periods" => "MedicationPausePeriod",
                "medication_takes" => "MedicationTake",
                "notification_preferences" => "NotificationPreference",
                "dose_occurrences" => "MedicationDoseOccurrence",
                "health_events" => "HealthEvent",
                _ => return Err(OperationError::Unavailable),
            };
            administration::persistence::record_version_as(
                tenant,
                record_type,
                id,
                "create",
                None,
                after,
                Some(provenance),
            )
            .await?;
            record_change(
                db,
                tenant,
                request_id,
                SyncRecord {
                    record_type,
                    record_id: id,
                    portable_id: &portable_id,
                    action: "create",
                    person_portable_id: record_person(kind, row, plan, index),
                },
            )
            .await?;
            if kind == "medication_takes" {
                super::writing::index_restored_take(db, tenant, provenance, id).await?;
            }
        }
        if kind == "medication_pause_periods" && plan.format.includes_extended_records() {
            reconcile_pauses(db, tenant, provenance, plan, index, request_id).await?;
        }
    }
    let mut created = Vec::with_capacity(expected.len());
    for record in expected {
        macro_rules! verify {
            ($id:expr, $expected:expr, $entity:ident, $variant:ident) => {{
                let row = $entity::Entity::find_by_id($id)
                    .one(db)
                    .await?
                    .ok_or(OperationError::Unavailable)?;
                if !same_fields($expected.as_ref(), &row.clone().into_active_model()) {
                    return Err(import_invalid("Restored record differs from the source"));
                }
                CreatedRecord::$variant(Box::new(row))
            }};
        }
        created.push(match record {
            ExpectedRecord::Person(id, value) => verify!(id, value, person, Person),
            ExpectedRecord::Location(id, value) => verify!(id, value, location, Location),
            ExpectedRecord::Medication(id, value) => verify!(id, value, medication, Medication),
            ExpectedRecord::DosageOption(id, value) => verify!(id, value, dosage, DosageOption),
            ExpectedRecord::Schedule(id, value) => verify!(id, value, schedule, Schedule),
            ExpectedRecord::Assignment(id, value) => {
                verify!(id, value, person_medication, Assignment)
            }
            ExpectedRecord::Pause(id, value) => verify!(id, value, pause_period, Pause),
            ExpectedRecord::Dose(id, value) => verify!(id, value, medication_take, Dose),
            ExpectedRecord::NotificationPreference(id, value) => {
                verify!(id, value, notification_preference, NotificationPreference)
            }
            ExpectedRecord::Occurrence(id, value) => {
                verify!(id, value, dose_occurrence, Occurrence)
            }
            ExpectedRecord::HealthEvent(id, value) => verify!(id, value, health_event, HealthEvent),
        });
    }
    for (event_id, expected_edges) in health_edges {
        let rows = health_event_medication::Entity::find()
            .filter(health_event_medication::Column::HouseholdId.eq(tenant.scope().household_id))
            .filter(health_event_medication::Column::HealthEventId.eq(event_id))
            .all(db)
            .await?;
        let mut actual_edges = Vec::new();
        for row in rows {
            actual_edges.push((
                row.medication_id
                    .ok_or_else(|| import_invalid("Restored health link is missing medication"))?,
                row.medication_name,
            ));
        }
        if !same_health_edges(expected_edges, actual_edges) {
            return Err(import_invalid(
                "Restored health links differ from the source",
            ));
        }
    }
    Ok(created)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_existing_portable_rows_before_create() {
        let plan = ImportPlan::parse(json!({"format":"medtracker.portable.v3", "records": {
            "locations":[{"portable_id":"location-1","name":"Cupboard"}],
            "medications":[{"portable_id":"medicine-1","location_portable_id":"location-1"}]
        }}))
        .unwrap();
        assert!(has_existing_record(&plan, |kind, id| kind == "medications"
            && id == "medicine-1"));
        assert!(!has_existing_record(&plan, |_, _| false));
    }

    #[test]
    fn checks_explicit_values_and_foreign_keys_but_skips_unset_fields() {
        let expected = schedule::ActiveModel {
            household_id: Set(1),
            person_id: Set(2),
            medication_id: Set(3),
            source_dosage_option_id: Set(Some(4)),
            active: Set(true),
            ..Default::default()
        };
        let mut actual = expected.clone();
        actual.notes = Set(Some("generated".into()));
        assert!(same_fields(&expected, &actual));
        actual.medication_id = Set(5);
        assert!(!same_fields(&expected, &actual));
        actual.medication_id = Set(3);
        actual.source_dosage_option_id = Set(Some(6));
        assert!(!same_fields(&expected, &actual));
        actual.source_dosage_option_id = Set(Some(4));
        actual.active = Set(false);
        assert!(!same_fields(&expected, &actual));
    }

    #[test]
    fn rejects_wrong_missing_or_duplicate_health_edges() {
        let expected = vec![(1, "A".into()), (2, "B".into())];
        assert!(same_health_edges(
            expected.clone(),
            vec![(2, "B".into()), (1, "A".into())]
        ));
        assert!(!same_health_edges(
            expected.clone(),
            vec![(1, "A".into()), (3, "B".into())]
        ));
        assert!(!same_health_edges(expected.clone(), vec![(1, "A".into())]));
        assert!(!same_health_edges(
            expected,
            vec![(1, "A".into()), (2, "B".into()), (2, "B".into())]
        ));
    }

    #[test]
    fn converts_document_timestamps_and_exact_decimal_values() {
        let fields = json!({"taken_at":"2020-01-02T10:30:00+02:00","dose_amount":"1.25"})
            .as_object()
            .unwrap()
            .clone();
        let result = document_fields("medication_takes", fields).unwrap();
        assert_eq!(result["taken_at"], "2020-01-02T08:30:00");
        assert_eq!(result["dose_amount"], "1.25");
        let fields = json!({"dose_amount":"0.1"}).as_object().unwrap().clone();
        assert!(document_fields("medications", fields).is_err());
    }
}
