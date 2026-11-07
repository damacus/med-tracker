use super::*;
use sea_orm::sea_query::{Alias, Expr, ExprTrait, JoinType, Query};

fn index_error_kind(error: &sea_orm::DbErr) -> &'static str {
    match error {
        sea_orm::DbErr::Type(_) => "type",
        sea_orm::DbErr::Query(_) => "query",
        sea_orm::DbErr::Exec(_) => "execution",
        sea_orm::DbErr::Conn(_) => "conn",
        sea_orm::DbErr::ConnectionAcquire(_) => "connection_acquire",
        sea_orm::DbErr::TryIntoErr { .. } => "try_into",
        sea_orm::DbErr::ConvertFromU64(_) => "convert_from_u64",
        sea_orm::DbErr::UnpackInsertId => "unpack_insert_id",
        sea_orm::DbErr::UpdateGetPrimaryKey => "update_get_primary_key",
        sea_orm::DbErr::RecordNotFound(_) => "record_not_found",
        sea_orm::DbErr::AttrNotSet(_) => "attr_not_set",
        sea_orm::DbErr::Custom(_) => "custom",
        sea_orm::DbErr::Json(_) => "json",
        sea_orm::DbErr::Migration(_) => "migration",
        sea_orm::DbErr::RecordNotInserted => "record_not_inserted",
        sea_orm::DbErr::RecordNotUpdated => "record_not_updated",
        sea_orm::DbErr::BackendNotSupported { .. } => "backend_not_supported",
        sea_orm::DbErr::KeyArityMismatch { .. } => "key_arity_mismatch",
        sea_orm::DbErr::PrimaryKeyNotSet { .. } => "primary_key_not_set",
        sea_orm::DbErr::RbacError(_) => "rbac_error",
        sea_orm::DbErr::AccessDenied { .. } => "access_denied",
        sea_orm::DbErr::MutexPoisonError => "mutex_poison",
        _ => "other",
    }
}

fn index_query_error(
    query: &'static str,
    table: &'static str,
    error: sea_orm::DbErr,
) -> OperationError {
    tracing::warn!(
        medtracker.import.index_query = query,
        medtracker.import.index_table = table,
        medtracker.import.index_column = "query",
        medtracker.import.index_error = index_error_kind(&error),
        medtracker.import.index_source_kind = "none",
        "Clinical import index failed"
    );
    error.into()
}

fn index_field<T: sea_orm::TryGetable>(
    row: &sea_orm::QueryResult,
    query: &'static str,
    table: &'static str,
    column: &'static str,
    source_kind: &'static str,
) -> Result<T, OperationError> {
    row.try_get_nullable("", column).map_err(|error| {
        let category = match &error {
            sea_orm::TryGetError::Null(_) => "null",
            sea_orm::TryGetError::DbErr(error) => index_error_kind(error),
        };
        tracing::warn!(
            medtracker.import.index_query = query,
            medtracker.import.index_table = table,
            medtracker.import.index_column = column,
            medtracker.import.index_error = category,
            medtracker.import.index_source_kind = source_kind,
            "Clinical import index failed"
        );
        OperationError::from(sea_orm::DbErr::from(error))
    })
}

pub(crate) struct ExistingIndex {
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
    pub(crate) async fn load(
        db: &DatabaseTransaction,
        household_id: i64,
    ) -> Result<Self, OperationError> {
        let mut ids = HashMap::new();
        let mut names = HashMap::new();
        for kind in BASE_TYPES.iter().chain(V2_TYPES) {
            let table = table(kind);
            let sql = if *kind == "people" {
                "SELECT p.id,p.portable_id,lower(p.email) AS match_name FROM public.people p JOIN public.household_memberships m ON m.person_id=p.id WHERE m.household_id=$1 AND m.status='active' AND m.revoked_at IS NULL".to_owned()
            } else if matches!(*kind, "locations" | "medications") {
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
                .map_err(|error| index_query_error("ids", kind, error))?;
            let mut type_ids = HashMap::new();
            let mut type_names = HashMap::new();
            for row in rows {
                let portable_id: String = index_field(&row, "ids", kind, "portable_id", "none")?;
                let id: i64 = index_field(&row, "ids", kind, "id", "none")?;
                if matches!(*kind, "locations" | "medications" | "people")
                    && let Ok(Some(name)) =
                        index_field::<Option<String>>(&row, "ids", kind, "match_name", "none")
                {
                    type_names.insert(name, portable_id.clone());
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
            let direct_owner = matches!(
                kind,
                "schedules" | "person_medications" | "notification_preferences" | "health_events"
            );
            let sources: &[(&str, &str)] = if direct_owner {
                &[]
            } else {
                &[
                    ("schedules", "schedule_id"),
                    ("person_medications", "person_medication_id"),
                ]
            };
            let mut type_owners = HashMap::new();
            for source in sources
                .iter()
                .copied()
                .map(Some)
                .chain(direct_owner.then_some(None))
            {
                let mut query = Query::select();
                query
                    .column(("record", "portable_id"))
                    .expr_as(Expr::col(("person", "portable_id")), "person_portable_id")
                    .from_as(Alias::new(table(kind)), "record")
                    .and_where(Expr::col(("record", "household_id")).eq(household_id));
                let owner_table = if let Some((source_table, source_column)) = source {
                    query.join_as(
                        JoinType::InnerJoin,
                        Alias::new(source_table),
                        "source",
                        Expr::col(("source", "id"))
                            .equals(("record", source_column))
                            .and(
                                Expr::col(("source", "household_id"))
                                    .equals(("record", "household_id")),
                            ),
                    );
                    "source"
                } else {
                    "record"
                };
                query.join_as(
                    JoinType::InnerJoin,
                    Alias::new("people"),
                    "person",
                    Expr::col(("person", "id")).equals((owner_table, "person_id")),
                );
                for row in db
                    .query_all_raw(db.get_database_backend().build(&query))
                    .await
                    .map_err(|error| index_query_error("owners", kind, error))?
                {
                    let portable_id: String =
                        index_field(&row, "owners", kind, "portable_id", "none")?;
                    let owner: String =
                        index_field(&row, "owners", kind, "person_portable_id", "none")?;
                    type_owners.insert(portable_id, owner);
                }
            }
            owners.insert(kind, type_owners);
        }
        let mut source_medications = HashMap::new();
        for kind in ["schedules", "person_medications"] {
            let sql = format!(
                "SELECT source.portable_id, medication.portable_id AS medication_portable_id FROM {} source JOIN medications medication ON medication.id = source.medication_id AND medication.household_id = source.household_id WHERE source.household_id = $1",
                table(kind)
            );
            let rows = db
                .query_all_raw(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    sql,
                    [household_id.into()],
                ))
                .await
                .map_err(|error| index_query_error("source_medications", kind, error))?;
            let mut mapped = HashMap::new();
            for row in rows {
                mapped.insert(
                    index_field(&row, "source_medications", kind, "portable_id", "none")?,
                    index_field(
                        &row,
                        "source_medications",
                        kind,
                        "medication_portable_id",
                        "none",
                    )?,
                );
            }
            source_medications.insert(kind, mapped);
        }
        let rows = db.query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT dosage.portable_id, medication.portable_id AS medication_portable_id, dosage.amount::text AS amount, dosage.unit FROM dosages dosage JOIN medications medication ON medication.id = dosage.medication_id AND medication.household_id = dosage.household_id WHERE dosage.household_id = $1", [household_id.into()]))
            .await.map_err(|error| index_query_error("dosage_values", "dosage_options", error))?;
        let mut dosage_medications = HashMap::new();
        let mut dosage_values = HashMap::new();
        for row in rows {
            let portable_id: String = index_field(
                &row,
                "dosage_values",
                "dosage_options",
                "portable_id",
                "none",
            )?;
            dosage_medications.insert(
                portable_id.clone(),
                index_field(
                    &row,
                    "dosage_values",
                    "dosage_options",
                    "medication_portable_id",
                    "none",
                )?,
            );
            dosage_values.insert(
                portable_id,
                (
                    index_field(&row, "dosage_values", "dosage_options", "amount", "none")?,
                    index_field(&row, "dosage_values", "dosage_options", "unit", "none")?,
                ),
            );
        }
        let rows = db.query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT medication.portable_id, location.portable_id AS location_portable_id FROM medications medication JOIN locations location ON location.id = medication.location_id AND location.household_id = medication.household_id WHERE medication.household_id = $1", [household_id.into()]))
            .await.map_err(|error| index_query_error("medication_locations", "medications", error))?;
        let mut medication_locations = HashMap::new();
        for row in rows {
            medication_locations.insert(
                index_field(
                    &row,
                    "medication_locations",
                    "medications",
                    "portable_id",
                    "none",
                )?,
                index_field(
                    &row,
                    "medication_locations",
                    "medications",
                    "location_portable_id",
                    "none",
                )?,
            );
        }
        let rows = db.query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT take.portable_id, CASE WHEN take.schedule_id IS NOT NULL THEN 'schedule' ELSE 'person_medication' END AS source_type, COALESCE(schedule.portable_id, assignment.portable_id) AS source_portable_id FROM medication_takes take LEFT JOIN schedules schedule ON schedule.id = take.schedule_id AND schedule.household_id = take.household_id LEFT JOIN person_medications assignment ON assignment.id = take.person_medication_id AND assignment.household_id = take.household_id WHERE take.household_id = $1", [household_id.into()]))
            .await.map_err(|error| index_query_error("take_sources", "medication_takes", error))?;
        let mut take_sources = HashMap::new();
        for row in rows {
            let portable_id = index_field(
                &row,
                "take_sources",
                "medication_takes",
                "portable_id",
                "none",
            )?;
            let source_type: String = index_field(
                &row,
                "take_sources",
                "medication_takes",
                "source_type",
                "none",
            )?;
            let source_kind = match source_type.as_str() {
                "schedule" => "schedule",
                "person_medication" => "person_medication",
                _ => "other",
            };
            let source_portable_id: Option<String> = index_field(
                &row,
                "take_sources",
                "medication_takes",
                "source_portable_id",
                source_kind,
            )?;
            if let Some(source_portable_id) = source_portable_id {
                take_sources.insert(portable_id, (source_type, source_portable_id));
            }
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

    pub(crate) fn get(&self, kind: &str, portable_id: &str) -> Option<i64> {
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
        if self.get("medication_takes", portable_id).is_some()
            && !self.take_sources.contains_key(portable_id)
        {
            return None;
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_take_with_unavailable_source_cannot_use_incoming_source() {
        let plan = ImportPlan::parse(json!({"format":"medtracker.portable.v3", "records": {
            "medication_takes":[{"portable_id":"retained", "source_type":"schedule", "source_portable_id":"spoofed"},
                                {"portable_id":"new", "source_type":"schedule", "source_portable_id":"new-source"}]
        }})).unwrap();
        let mut index = ExistingIndex {
            ids: HashMap::from([(
                "medication_takes",
                HashMap::from([("retained".to_owned(), 1)]),
            )]),
            names: HashMap::new(),
            owners: HashMap::new(),
            source_medications: HashMap::new(),
            dosage_medications: HashMap::new(),
            dosage_values: HashMap::new(),
            medication_locations: HashMap::new(),
            take_sources: HashMap::new(),
        };
        assert_eq!(index.get("medication_takes", "retained"), Some(1));
        assert_eq!(index.take_source(&plan, "retained"), None);
        assert_eq!(
            index.take_source(&plan, "new"),
            Some(("schedule", "new-source"))
        );
        index
            .take_sources
            .insert("visible".into(), ("schedule".into(), "source".into()));
        assert_eq!(
            index.take_source(&plan, "visible"),
            Some(("schedule", "source"))
        );
    }
}
