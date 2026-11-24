use super::*;
use crate::views::{
    PaginationMeta,
    treatments::{PauseSourceKind, PauseSummary, TreatmentCollection, TreatmentResponse},
};
use loco_rs::model::TenantQueryExt;
use sha2::{Digest, Sha256};
#[derive(Default, Deserialize)]
pub struct Pagination {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub source_type: Option<String>,
    pub source_id: Option<String>,
}
async fn names(
    db: &DatabaseTransaction,
    periods: &[pause_period::Model],
) -> Result<HashMap<i64, String>, OperationError> {
    let ids: Vec<i64> = periods
        .iter()
        .flat_map(|row| [row.recorded_by_membership_id, row.resumed_by_membership_id])
        .flatten()
        .collect();
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let members = membership::Entity::find()
        .filter(membership::Column::Id.is_in(ids))
        .all(db)
        .await?;
    let people: HashMap<i64, String> = person::Entity::find()
        .filter(
            person::Column::Id.is_in(members.iter().map(|row| row.person_id).collect::<Vec<_>>()),
        )
        .all(db)
        .await?
        .into_iter()
        .map(|row| (row.id, row.name))
        .collect();
    Ok(members
        .into_iter()
        .filter_map(|row| {
            people
                .get(&row.person_id)
                .cloned()
                .map(|name| (row.id, name))
        })
        .collect())
}
fn value(row: &pause_period::Model, source: &Source, names: &HashMap<i64, String>) -> PauseSummary {
    PauseSummary {
        id: row.portable_id.clone(),
        portable_id: row.portable_id.clone(),
        source_type: match source {
            Source::Schedule(_) => PauseSourceKind::Schedule,
            Source::Assignment(_) => PauseSourceKind::PersonMedication,
        },
        source_id: source.portable_id().to_owned(),
        reason: row.reason.clone(),
        note: row.note.clone(),
        legacy_context: row.legacy_context,
        started_at: row.started_at.map(timestamp),
        ended_at: row.ended_at.map(timestamp),
        recorded_by_membership_id: row.recorded_by_membership_id.map(|id| id.to_string()),
        resumed_by_membership_id: row.resumed_by_membership_id.map(|id| id.to_string()),
        recorded_by_name: row
            .recorded_by_membership_id
            .and_then(|id| names.get(&id))
            .cloned(),
        resumed_by_name: row
            .resumed_by_membership_id
            .and_then(|id| names.get(&id))
            .cloned(),
        created_at: timestamp(row.created_at),
        updated_at: timestamp(row.updated_at),
    }
}
type VersionedPause = (TreatmentResponse<PauseSummary>, String);

pub(super) async fn project(
    tenant: &TenantTransaction,
    row: &pause_period::Model,
    source: &Source,
) -> Result<VersionedPause, OperationError> {
    let names = names(tenant.transaction(), std::slice::from_ref(row)).await?;
    representation(value(row, source, &names))
}
pub(crate) async fn period_values(
    db: &DatabaseTransaction,
    periods: &[pause_period::Model],
    schedules: &HashMap<i64, schedule::Model>,
    assignments: &HashMap<i64, person_medication::Model>,
) -> Result<Vec<VersionedPause>, OperationError> {
    let names = names(db, periods).await?;
    periods
        .iter()
        .map(|row| {
            let source = if let Some(id) = row.schedule_id {
                Source::Schedule(
                    schedules
                        .get(&id)
                        .cloned()
                        .ok_or(OperationError::NotFound)?,
                )
            } else {
                Source::Assignment(
                    assignments
                        .get(&row.person_medication_id.ok_or(OperationError::NotFound)?)
                        .cloned()
                        .ok_or(OperationError::NotFound)?,
                )
            };
            representation(value(row, &source, &names))
        })
        .collect()
}
fn representation(data: PauseSummary) -> Result<VersionedPause, OperationError> {
    let body = TreatmentResponse { data };
    let mut canonical = serde_json::to_value(&body).map_err(|_| OperationError::Unavailable)?;
    canonical.sort_all_objects();
    let etag = format!(
        "\"{}\"",
        hex::encode(Sha256::digest(canonical.to_string().as_bytes()))
    );
    Ok((body, etag))
}
pub async fn list(
    tenant: &TenantTransaction,
    page: Pagination,
) -> Result<TreatmentCollection<PauseSummary>, OperationError> {
    access::recheck(tenant).await?;
    let number = page.page.unwrap_or(1);
    let size = page.per_page.unwrap_or(20);
    if number < 1 || !(1..=100).contains(&size) {
        return Err(invalid("pagination", "is invalid"));
    }
    let selected = match (page.source_type.as_deref(), page.source_id.as_deref()) {
        (None, None) => None,
        (Some(kind), Some(id)) => {
            let kind = Kind::parse(kind).ok_or_else(|| invalid("source_type", "is invalid"))?;
            Uuid::parse_str(id).map_err(|_| invalid("source_id", "is invalid"))?;
            Some(find_source(tenant, kind, id, true).await?)
        }
        _ => {
            return Err(invalid(
                "source",
                "source_type and source_id are required together",
            ));
        }
    };
    let schedules = schedule::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .filter(
            schedule::Column::PersonId
                .in_subquery(access::participating_people(tenant.membership())),
        )
        .all(tenant.transaction())
        .await?;
    let assignments = person_medication::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .filter(
            person_medication::Column::PersonId
                .in_subquery(access::participating_people(tenant.membership())),
        )
        .all(tenant.transaction())
        .await?;
    let mut sources = HashMap::new();
    for row in schedules {
        sources.insert(("schedule", row.id), Source::Schedule(row));
    }
    for row in assignments {
        sources.insert(("person_medication", row.id), Source::Assignment(row));
    }
    let schedules: Vec<i64> = sources
        .keys()
        .filter_map(|(kind, id)| (*kind == "schedule").then_some(*id))
        .collect();
    let assignments: Vec<i64> = sources
        .keys()
        .filter_map(|(kind, id)| (*kind == "person_medication").then_some(*id))
        .collect();
    let mut query = pause_period::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .filter(
            Condition::any()
                .add(pause_period::Column::ScheduleId.is_in(schedules))
                .add(pause_period::Column::PersonMedicationId.is_in(assignments)),
        );
    if let Some(source) = selected {
        query = match source {
            Source::Schedule(row) => query.filter(pause_period::Column::ScheduleId.eq(row.id)),
            Source::Assignment(row) => {
                query.filter(pause_period::Column::PersonMedicationId.eq(row.id))
            }
        };
    }
    let total = query.clone().count(tenant.transaction()).await?;
    let rows = query
        .order_by_desc(pause_period::Column::CreatedAt)
        .order_by_desc(pause_period::Column::Id)
        .limit(size as u64)
        .offset(number.saturating_sub(1).saturating_mul(size) as u64)
        .all(tenant.transaction())
        .await?;
    let names = names(tenant.transaction(), &rows).await?;
    let data = rows
        .iter()
        .filter_map(|row| {
            let key = if let Some(id) = row.schedule_id {
                ("schedule", id)
            } else {
                ("person_medication", row.person_medication_id?)
            };
            sources.get(&key).map(|source| value(row, source, &names))
        })
        .collect();
    Ok(TreatmentCollection {
        data,
        meta: PaginationMeta {
            page: number,
            per_page: size,
            total_count: total,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pause_response_preserves_canonical_etag() -> Result<(), OperationError> {
        let (response, etag) = representation(PauseSummary {
            id: "pause-id".into(),
            portable_id: "pause-id".into(),
            source_type: PauseSourceKind::Schedule,
            source_id: "source-id".into(),
            reason: "other".into(),
            note: None,
            legacy_context: false,
            started_at: None,
            ended_at: None,
            recorded_by_membership_id: Some("42".into()),
            resumed_by_membership_id: None,
            recorded_by_name: Some("Recorder".into()),
            resumed_by_name: None,
            created_at: "2026-11-16T00:00:00.000Z".into(),
            updated_at: "2026-11-16T00:00:00.000Z".into(),
        })?;
        assert_eq!(
            response.data.recorded_by_membership_id.as_deref(),
            Some("42")
        );
        assert_eq!(
            etag,
            "\"1638c503a54e1e10cbeeb41b7d3024b9d97798b6df4184c9ff57fd2608523f77\""
        );
        Ok(())
    }
}
