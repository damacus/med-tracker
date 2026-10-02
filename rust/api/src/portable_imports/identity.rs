use super::*;

pub(super) struct ExistingIndex {
    pub(super) ids: HashMap<&'static str, HashMap<String, i64>>,
    pub(super) names: HashMap<&'static str, HashMap<String, String>>,
    pub(super) owners: HashMap<&'static str, HashMap<String, String>>,
    pub(super) source_medications: HashMap<&'static str, HashMap<String, String>>,
    pub(super) dosage_medications: HashMap<String, String>,
    dosage_values: HashMap<String, (String, String)>,
    medication_locations: HashMap<String, String>,
    take_sources: HashMap<String, (String, String)>,
}

impl ExistingIndex {
    pub(super) async fn load(
        db: &DatabaseTransaction,
        household_id: i64,
    ) -> Result<Self, ApiError> {
        let mut ids = HashMap::new();
        let mut names = HashMap::new();
        for kind in BASE_TYPES.iter().chain(V2_TYPES) {
            let table = table(kind);
            let sql = if matches!(*kind, "locations" | "medications" | "people") {
                format!(
                    "SELECT id, portable_id, {} AS match_name FROM {table} WHERE household_id = $1",
                    if *kind == "people" {
                        "lower(email)"
                    } else {
                        "lower(name)"
                    }
                )
            } else {
                format!("SELECT id, portable_id FROM {table} WHERE household_id = $1")
            };
            let rows = db
                .query_all_raw(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    sql,
                    [household_id.into()],
                ))
                .await
                .map_err(database_error)?;
            let mut type_ids = HashMap::new();
            let mut type_names = HashMap::new();
            for row in rows {
                let portable_id: String = row.try_get("", "portable_id").map_err(database_error)?;
                let id: i64 = row.try_get("", "id").map_err(database_error)?;
                if matches!(*kind, "locations" | "medications" | "people") {
                    if let Ok(Some(name)) = row.try_get::<Option<String>>("", "match_name") {
                        type_names.insert(name, portable_id.clone());
                    }
                }
                type_ids.insert(portable_id, id);
            }
            ids.insert(*kind, type_ids);
            names.insert(*kind, type_names);
        }
        let mut owners = HashMap::new();
        for kind in [
            "schedules",
            "person_medications",
            "notification_preferences",
            "health_events",
            "medication_takes",
            "medication_pause_periods",
            "dose_occurrences",
        ] {
            let table = table(kind);
            let sql = if matches!(
                kind,
                "schedules" | "person_medications" | "notification_preferences" | "health_events"
            ) {
                format!("SELECT record.portable_id, person.portable_id AS person_portable_id FROM {table} record JOIN people person ON person.id = record.person_id AND person.household_id = record.household_id WHERE record.household_id = $1")
            } else {
                format!("SELECT record.portable_id, person.portable_id AS person_portable_id FROM {table} record LEFT JOIN schedules schedule ON schedule.id = record.schedule_id AND schedule.household_id = record.household_id LEFT JOIN person_medications assignment ON assignment.id = record.person_medication_id AND assignment.household_id = record.household_id JOIN people person ON person.id = COALESCE(schedule.person_id, assignment.person_id) AND person.household_id = record.household_id WHERE record.household_id = $1")
            };
            let rows = db
                .query_all_raw(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    sql,
                    [household_id.into()],
                ))
                .await
                .map_err(database_error)?;
            let mut type_owners = HashMap::new();
            for row in rows {
                let portable_id: String = row.try_get("", "portable_id").map_err(database_error)?;
                let owner: String = row
                    .try_get("", "person_portable_id")
                    .map_err(database_error)?;
                type_owners.insert(portable_id, owner);
            }
            owners.insert(kind, type_owners);
        }
        let mut source_medications = HashMap::new();
        for kind in ["schedules", "person_medications"] {
            let sql = format!("SELECT source.portable_id, medication.portable_id AS medication_portable_id FROM {} source JOIN medications medication ON medication.id = source.medication_id AND medication.household_id = source.household_id WHERE source.household_id = $1", table(kind));
            let rows = db
                .query_all_raw(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    sql,
                    [household_id.into()],
                ))
                .await
                .map_err(database_error)?;
            let mut mapped = HashMap::new();
            for row in rows {
                mapped.insert(
                    row.try_get("", "portable_id").map_err(database_error)?,
                    row.try_get("", "medication_portable_id")
                        .map_err(database_error)?,
                );
            }
            source_medications.insert(kind, mapped);
        }
        let rows = db.query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT dosage.portable_id, medication.portable_id AS medication_portable_id, dosage.amount::text AS amount, dosage.unit FROM dosages dosage JOIN medications medication ON medication.id = dosage.medication_id AND medication.household_id = dosage.household_id WHERE dosage.household_id = $1", [household_id.into()]))
            .await.map_err(database_error)?;
        let mut dosage_medications = HashMap::new();
        let mut dosage_values = HashMap::new();
        for row in rows {
            let portable_id: String = row.try_get("", "portable_id").map_err(database_error)?;
            dosage_medications.insert(
                portable_id.clone(),
                row.try_get("", "medication_portable_id")
                    .map_err(database_error)?,
            );
            dosage_values.insert(
                portable_id,
                (
                    row.try_get("", "amount").map_err(database_error)?,
                    row.try_get("", "unit").map_err(database_error)?,
                ),
            );
        }
        let rows = db.query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT medication.portable_id, location.portable_id AS location_portable_id FROM medications medication JOIN locations location ON location.id = medication.location_id AND location.household_id = medication.household_id WHERE medication.household_id = $1", [household_id.into()]))
            .await.map_err(database_error)?;
        let mut medication_locations = HashMap::new();
        for row in rows {
            medication_locations.insert(
                row.try_get("", "portable_id").map_err(database_error)?,
                row.try_get("", "location_portable_id")
                    .map_err(database_error)?,
            );
        }
        let rows = db.query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT take.portable_id, CASE WHEN take.schedule_id IS NOT NULL THEN 'schedule' ELSE 'person_medication' END AS source_type, COALESCE(schedule.portable_id, assignment.portable_id) AS source_portable_id FROM medication_takes take LEFT JOIN schedules schedule ON schedule.id = take.schedule_id AND schedule.household_id = take.household_id LEFT JOIN person_medications assignment ON assignment.id = take.person_medication_id AND assignment.household_id = take.household_id WHERE take.household_id = $1", [household_id.into()]))
            .await.map_err(database_error)?;
        let mut take_sources = HashMap::new();
        for row in rows {
            take_sources.insert(
                row.try_get("", "portable_id").map_err(database_error)?,
                (
                    row.try_get("", "source_type").map_err(database_error)?,
                    row.try_get("", "source_portable_id")
                        .map_err(database_error)?,
                ),
            );
        }
        Ok(Self {
            ids,
            names,
            owners,
            source_medications,
            dosage_medications,
            dosage_values,
            medication_locations,
            take_sources,
        })
    }

    pub(super) fn has(&self, plan: &ImportPlan, kind: &str, portable_id: &str) -> bool {
        self.ids
            .get(kind)
            .is_some_and(|ids| ids.contains_key(portable_id))
            || plan
                .rows(kind)
                .iter()
                .any(|row| row["portable_id"] == portable_id)
    }

    pub(super) fn get(&self, kind: &str, portable_id: &str) -> Option<i64> {
        self.ids
            .get(kind)
            .and_then(|ids| ids.get(portable_id))
            .copied()
    }

    pub(super) fn put(&mut self, kind: &'static str, portable_id: String, id: i64) {
        self.ids.entry(kind).or_default().insert(portable_id, id);
    }

    pub(super) fn owner(&self, kind: &str, portable_id: &str) -> Option<&str> {
        self.owners
            .get(kind)
            .and_then(|owners| owners.get(portable_id))
            .map(String::as_str)
    }

    pub(super) fn source_medication<'a>(
        &'a self,
        plan: &'a ImportPlan,
        kind: &str,
        portable_id: &str,
    ) -> Option<&'a str> {
        plan.rows(kind)
            .iter()
            .find(|row| row["portable_id"] == portable_id)
            .and_then(|row| row["medication_portable_id"].as_str())
            .or_else(|| {
                self.source_medications
                    .get(kind)
                    .and_then(|rows| rows.get(portable_id).map(String::as_str))
            })
    }

    pub(super) fn dosage_medication<'a>(
        &'a self,
        plan: &'a ImportPlan,
        portable_id: &str,
    ) -> Option<&'a str> {
        plan.rows("dosage_options")
            .iter()
            .find(|row| row["portable_id"] == portable_id)
            .and_then(|row| row["medication_portable_id"].as_str())
            .or_else(|| self.dosage_medications.get(portable_id).map(String::as_str))
    }

    pub(super) fn dosage_value<'a>(
        &'a self,
        plan: &'a ImportPlan,
        portable_id: &str,
    ) -> Option<(Decimal, &'a str)> {
        if let Some(row) = plan
            .rows("dosage_options")
            .iter()
            .find(|row| row["portable_id"] == portable_id)
        {
            return Some((decimal(&row["amount"])?, row["unit"].as_str()?));
        }
        let (amount, unit) = self.dosage_values.get(portable_id)?;
        Some((Decimal::from_str(amount).ok()?, unit))
    }

    pub(super) fn medication_location<'a>(
        &'a self,
        plan: &'a ImportPlan,
        portable_id: &str,
    ) -> Option<&'a str> {
        plan.rows("medications")
            .iter()
            .find(|row| row["portable_id"] == portable_id)
            .and_then(|row| row["location_portable_id"].as_str())
            .or_else(|| {
                self.medication_locations
                    .get(portable_id)
                    .map(String::as_str)
            })
    }

    pub(super) fn take_source<'a>(
        &'a self,
        plan: &'a ImportPlan,
        portable_id: &str,
    ) -> Option<(&'a str, &'a str)> {
        if let Some(row) = plan
            .rows("medication_takes")
            .iter()
            .find(|row| row["portable_id"] == portable_id)
        {
            return Some((
                row["source_type"].as_str()?,
                row["source_portable_id"].as_str()?,
            ));
        }
        self.take_sources
            .get(portable_id)
            .map(|(kind, id)| (kind.as_str(), id.as_str()))
    }
}

pub(super) fn table(kind: &str) -> &'static str {
    match kind {
        "people" => "people",
        "locations" => "locations",
        "medications" => "medications",
        "dosage_options" => "dosages",
        "schedules" => "schedules",
        "person_medications" => "person_medications",
        "medication_takes" => "medication_takes",
        "notification_preferences" => "notification_preferences",
        "medication_pause_periods" => "medication_pause_periods",
        "dose_occurrences" => "medication_dose_occurrences",
        "health_events" => "health_events",
        _ => unreachable!(),
    }
}
