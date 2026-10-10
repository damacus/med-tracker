use super::*;
use crate::models::entities::health_event_medication;
use std::collections::HashMap;

pub async fn collection(
    tenant: &TenantTransaction,
    person_id: i64,
) -> Result<Vec<Value>, OperationError> {
    access::recheck(tenant).await?;
    access::require_person_access(tenant, person_id, PersonAccess::View).await?;
    let rows = health_event::Entity::find()
        .filter(health_event::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(health_event::Column::PersonId.eq(person_id))
        .order_by_desc(health_event::Column::StartedOn)
        .order_by_desc(health_event::Column::Id)
        .all(tenant.transaction())
        .await?;
    if rows.is_empty() {
        return Ok(Vec::new());
    }
    let links = health_event_medication::Entity::find()
        .filter(health_event_medication::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(health_event_medication::Column::HealthEventId.is_in(rows.iter().map(|row| row.id)))
        .order_by_asc(health_event_medication::Column::Id)
        .all(tenant.transaction())
        .await?;
    let mut links_by_event: HashMap<i64, Vec<health_event_medication::Model>> = HashMap::new();
    for link in links {
        links_by_event
            .entry(link.health_event_id)
            .or_default()
            .push(link);
    }
    Ok(rows.into_iter().map(|row| {
        let links = links_by_event.remove(&row.id).unwrap_or_default();
        let etag = sync::health_events::browser_etag(&row,&links);
        let medication_names = links.into_iter().map(|link|link.medication_name).collect::<Vec<_>>();
        json!({
            "id":row.id,"portable_id":row.portable_id,"title":row.title,
            "event_kind":if row.event_kind==0 {"illness"} else {"suspected_side_effect"},
            "severity":row.severity.map(|severity| match severity {0=>"mild",1=>"moderate",_=>"severe"}),
            "started_on":row.started_on,"ended_on":row.ended_on,"ongoing":row.ended_on.is_none(),
            "notes":row.notes,"action_taken":row.action_taken,"medical_help_sought":row.medical_help_sought,
            "medication_names":medication_names,"etag":etag
        })
    }).collect())
}

pub async fn event(
    tenant: &TenantTransaction,
    person_id: i64,
    id: &str,
) -> Result<
    (
        health_event::Model,
        Vec<health_event_medication::Model>,
        String,
    ),
    OperationError,
> {
    let row = super::selected(tenant, id).await?;
    if row.person_id != person_id {
        return Err(OperationError::NotFound);
    }
    let links = health_event_medication::Entity::find()
        .filter(health_event_medication::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(health_event_medication::Column::HealthEventId.eq(row.id))
        .order_by_asc(health_event_medication::Column::Id)
        .all(tenant.transaction())
        .await?;
    let etag = sync::health_events::browser_etag(&row, &links);
    Ok((row, links, etag))
}

pub async fn medication_options(
    tenant: &TenantTransaction,
    person_id: i64,
) -> Result<Vec<crate::models::entities::medication::Model>, OperationError> {
    use crate::models::entities::{medication, person_medication, schedule};
    use sea_orm::{Condition, QueryTrait};
    access::recheck(tenant).await?;
    access::require_person_access(tenant, person_id, PersonAccess::View).await?;
    let schedules = schedule::Entity::find()
        .select_only()
        .column(schedule::Column::MedicationId)
        .filter(schedule::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(schedule::Column::PersonId.eq(person_id))
        .into_query();
    let assignments = person_medication::Entity::find()
        .select_only()
        .column(person_medication::Column::MedicationId)
        .filter(person_medication::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(person_medication::Column::PersonId.eq(person_id))
        .into_query();
    access::medication_scope(tenant)
        .filter(
            Condition::any()
                .add(medication::Column::Id.in_subquery(schedules))
                .add(medication::Column::Id.in_subquery(assignments)),
        )
        .order_by_asc(medication::Column::Name)
        .order_by_asc(medication::Column::Id)
        .all(tenant.transaction())
        .await
        .map_err(Into::into)
}

fn invalid(field: &str) -> OperationError {
    OperationError::Validation {
        details: json!({"errors":{field:["is invalid"]}}),
    }
}

pub async fn save(
    tenant: &TenantTransaction,
    person_id: i64,
    id: Option<&str>,
    draft: &HashMap<String, String>,
    provenance: &CredentialProvenance,
) -> Result<Value, OperationError> {
    sync::lock(tenant).await?;
    access::require_person_access(
        tenant,
        person_id,
        if id.is_some() {
            PersonAccess::Manage
        } else {
            PersonAccess::Record
        },
    )
    .await?;
    let if_match = if let Some(id) = id {
        let (row, _, etag) = event(tenant, person_id, id).await?;
        if draft.get("etag").map(String::as_str) != Some(etag.as_str()) {
            return Err(OperationError::Conflict {
                code: "sync_conflict".into(),
                details: json!({"error":"Record has changed since it was last read"}),
            });
        }
        Some(
            sync::health_events::representation(tenant.transaction(), &row)
                .await?
                .1,
        )
    } else {
        None
    };
    let field = |key: &str| draft.get(key).map_or("", String::as_str);
    if !matches!(field("ongoing"), "" | "1") || !matches!(field("medical_help_sought"), "" | "1") {
        return Err(invalid("health_event"));
    }
    let mut ids = std::collections::HashSet::new();
    for (key, value) in draft {
        if let Some(id) = key.strip_prefix("medication_") {
            if value != "1" {
                return Err(invalid("medication_ids"));
            }
            let id = id.parse::<i64>().map_err(|_| invalid("medication_ids"))?;
            if id <= 0 || !ids.insert(id) {
                return Err(invalid("medication_ids"));
            }
        }
    }
    let options = medication_options(tenant, person_id).await?;
    let editable_medication_ids = options.iter().map(|row| row.id).collect();
    let medications = options
        .into_iter()
        .filter(|row| ids.contains(&row.id))
        .collect::<Vec<_>>();
    if medications.len() != ids.len()
        || (field("event_kind") != "suspected_side_effect" && !ids.is_empty())
    {
        return Err(invalid("medication_ids"));
    }
    let ongoing = field("ongoing") == "1" || field("ended_on").is_empty();
    let mut attributes = serde_json::Map::new();
    for name in ["event_kind", "title", "started_on", "notes"] {
        attributes.insert(name.into(), json!(field(name)));
    }
    attributes.insert("person_id".into(), json!(person_id.to_string()));
    if !field("severity").is_empty() {
        attributes.insert("severity".into(), json!(field("severity")));
    }
    if !ongoing {
        attributes.insert("ended_on".into(), json!(field("ended_on")));
    }
    let input = sync::health_events::BrowserInput {
        person_id,
        medications,
        editable_medication_ids,
        action_taken: (!field("action_taken").trim().is_empty())
            .then(|| field("action_taken").to_owned()),
        medical_help_sought: field("medical_help_sought") == "1",
        ongoing,
    };
    sync::health_events::apply_browser(
        tenant,
        &sync::Operation {
            resource_type: "health_event".into(),
            action: if id.is_some() { "update" } else { "create" }.into(),
            id: id.map(str::to_owned),
            if_match,
            attributes,
        },
        &input,
        provenance,
    )
    .await
}

pub async fn delete(
    tenant: &TenantTransaction,
    person_id: i64,
    id: &str,
    expected: &str,
    provenance: &CredentialProvenance,
) -> Result<(), OperationError> {
    sync::lock(tenant).await?;
    access::require_person_access(tenant, person_id, PersonAccess::Manage).await?;
    let (row, _, etag) = event(tenant, person_id, id).await?;
    if expected != etag {
        return Err(OperationError::Conflict {
            code: "sync_conflict".into(),
            details: json!({"error":"Record has changed since it was last read"}),
        });
    }
    let (_, api_etag) = sync::health_events::representation(tenant.transaction(), &row).await?;
    sync::health_events::apply(
        tenant,
        &sync::Operation {
            resource_type: "health_event".into(),
            action: "delete".into(),
            id: Some(id.into()),
            if_match: Some(api_etag),
            attributes: serde_json::Map::new(),
        },
        provenance,
    )
    .await?;
    Ok(())
}
