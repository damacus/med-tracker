use super::*;
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
            person::Column::Id.is_in(
                members
                    .iter()
                    .filter_map(|row| row.person_id)
                    .collect::<Vec<_>>(),
            ),
        )
        .all(db)
        .await?
        .into_iter()
        .map(|row| (row.id, row.name))
        .collect();
    Ok(members
        .into_iter()
        .filter_map(|row| {
            row.person_id
                .and_then(|id| people.get(&id).cloned())
                .map(|name| (row.id, name))
        })
        .collect())
}
fn value(row: &pause_period::Model, source: &Source, names: &HashMap<i64, String>) -> Value {
    json!({"id":row.portable_id,"portable_id":row.portable_id,"source_type":source.kind().name(),"source_id":source.portable_id(),"reason":row.reason,"note":row.note,"legacy_context":row.legacy_context,"started_at":row.started_at.map(timestamp),"ended_at":row.ended_at.map(timestamp),"recorded_by_membership_id":row.recorded_by_membership_id.map(|id|id.to_string()),"resumed_by_membership_id":row.resumed_by_membership_id.map(|id|id.to_string()),"recorded_by_name":row.recorded_by_membership_id.and_then(|id|names.get(&id)),"resumed_by_name":row.resumed_by_membership_id.and_then(|id|names.get(&id)),"created_at":timestamp(row.created_at),"updated_at":timestamp(row.updated_at)})
}
pub(super) async fn project(
    tenant: &TenantTransaction,
    row: &pause_period::Model,
    source: &Source,
) -> Result<(Value, String), OperationError> {
    let names = names(tenant.transaction(), std::slice::from_ref(row)).await?;
    let mut body = json!({"data":value(row,source,&names)});
    body.sort_all_objects();
    let etag = format!(
        "\"{}\"",
        hex::encode(Sha256::digest(body.to_string().as_bytes()))
    );
    Ok((body, etag))
}
pub async fn list(tenant: &TenantTransaction, page: Pagination) -> Result<Value, OperationError> {
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
        .filter(schedule::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(schedule::Column::PersonId.in_subquery(access::granted_people(tenant.membership())))
        .all(tenant.transaction())
        .await?;
    let assignments = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(
            person_medication::Column::PersonId
                .in_subquery(access::granted_people(tenant.membership())),
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
        .filter(pause_period::Column::HouseholdId.eq(tenant.scope().household_id))
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
    let data: Vec<Value> = rows
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
    Ok(json!({"data":data,"meta":{"page":number,"per_page":size,"total_count":total}}))
}
