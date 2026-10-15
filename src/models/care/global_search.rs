use crate::models::{
    access::{self, TenantTransaction},
    care::{locations, medications, people, reports},
    entities::{medication, person_medication, schedule},
    errors::OperationError,
};
use chrono::NaiveDate;
use loco_rs::model::TenantQueryExt;
use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

const LIMIT: usize = 12;

#[derive(Clone, Serialize)]
pub struct SearchResult {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub title: String,
    pub subtitle: String,
    pub path: String,
    pub score: i32,
}

impl SearchResult {
    fn new(
        kind: &'static str,
        title: String,
        subtitle: String,
        path: String,
        query: &str,
        secondary: &[&str],
    ) -> Self {
        let title_match = title.trim().to_lowercase();
        let query_match = query.trim().to_lowercase();
        let score = if title_match == query_match {
            100
        } else if title_match.starts_with(&query_match) {
            80
        } else if title_match.contains(&query_match) {
            60
        } else if secondary
            .iter()
            .any(|value| value.trim().to_lowercase().contains(&query_match))
        {
            40
        } else {
            0
        };
        Self {
            kind,
            title,
            subtitle,
            path,
            score,
        }
    }
}

pub async fn query(
    tenant: &TenantTransaction,
    slug: &str,
    query: &str,
    labels: &Value,
    today: NaiveDate,
) -> Result<Vec<SearchResult>, OperationError> {
    access::recheck(tenant).await?;
    let query = query.trim();
    let mut results = commands(tenant, slug, labels, today).await?;
    if query.is_empty() {
        results.truncate(LIMIT);
        return Ok(results);
    }
    results.retain_mut(|result| {
        result.score = SearchResult::new(
            result.kind,
            result.title.clone(),
            result.subtitle.clone(),
            result.path.clone(),
            query,
            &[&result.subtitle],
        )
        .score;
        result.score > 0
    });
    if query.chars().count() >= 2 {
        results.extend(records(tenant, slug, query, labels).await?);
    }
    results.retain(|result| result.score > 0);
    results.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| kind_order(a.kind).cmp(&kind_order(b.kind)))
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
    });
    results.truncate(LIMIT);
    Ok(results)
}

async fn commands(
    tenant: &TenantTransaction,
    slug: &str,
    labels: &Value,
    today: NaiveDate,
) -> Result<Vec<SearchResult>, OperationError> {
    let can_create = medications::crud::can_create(tenant).await?;
    let can_manage_person = access::has_participants(tenant).await?;
    let can_create_person = people::can_create(tenant).await?;
    let can_view_person = access::has_participants(tenant).await?;
    let manager = access::can_manage_household(tenant);
    let can_report = manager || reports::actor_adult(tenant, today).await?;
    let mut results = Vec::new();
    let definitions = [
        (
            "medication_finder",
            "medications/finder",
            can_create || manager || can_view_person,
        ),
        ("inventory", "medications", true),
        ("people", "people", true),
        ("locations", "locations", true),
        ("reports", "reports", can_report),
        ("add_medication", "medications/workflow", can_create),
        ("add_person", "people/new", can_create_person),
        (
            "schedule_workflow",
            "medications/workflow?intent=schedule",
            can_manage_person,
        ),
    ];
    for (key, path, allowed) in definitions {
        if !allowed {
            continue;
        }
        let command = &labels["global_search"]["commands"][key];
        results.push(SearchResult {
            kind: "command",
            title: command["title"].as_str().unwrap_or_default().to_owned(),
            subtitle: command["subtitle"].as_str().unwrap_or_default().to_owned(),
            path: format!("/households/{slug}/{path}"),
            score: 50,
        });
    }
    Ok(results)
}

async fn records(
    tenant: &TenantTransaction,
    slug: &str,
    query: &str,
    labels: &Value,
) -> Result<Vec<SearchResult>, OperationError> {
    let household_id = tenant.scope().household_id;
    let term = format!("%{}%", escape_pattern(query));
    let people = crate::models::person_identity::current_summaries(tenant).await?;
    let people_by_id: HashMap<_, _> = people.iter().map(|row| (row.id.get(), row)).collect();
    let matching_people: Vec<_> = people
        .iter()
        .filter(|row| row.name.to_lowercase().contains(&query.to_lowercase()))
        .collect();
    let mut results = Vec::new();
    for row in matching_people.iter().take(LIMIT) {
        results.push(SearchResult::new(
            "person",
            row.name.clone(),
            label(labels, "person"),
            format!("/households/{slug}/people/{}", row.id),
            query,
            &[],
        ));
    }
    let medicines = access::medication_scope(tenant)
        .filter(
            Condition::any()
                .add(medication::Column::Name.ilike(&term))
                .add(medication::Column::FriendlyName.ilike(&term))
                .add(medication::Column::Category.ilike(&term))
                .add(medication::Column::Barcode.ilike(&term))
                .add(medication::Column::DmdCode.ilike(&term)),
        )
        .order_by_asc(medication::Column::Name)
        .limit(LIMIT as u64)
        .all(tenant.transaction())
        .await?;
    let locations = locations::list(tenant).await?;
    let location_names: HashMap<_, _> = locations
        .iter()
        .map(|row| (row.id, row.name.as_str()))
        .collect();
    for row in &medicines {
        let subtitle = [
            row.category.as_deref(),
            row.location_id
                .and_then(|id| location_names.get(&id))
                .copied(),
        ]
        .into_iter()
        .flatten()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
        let subtitle = if subtitle.is_empty() {
            label(labels, "medication")
        } else {
            subtitle
        };
        results.push(SearchResult::new(
            "medication",
            medication_name(row),
            subtitle,
            format!("/households/{slug}/medications/{}", row.id),
            query,
            &[
                row.name.as_deref().unwrap_or_default(),
                row.category.as_deref().unwrap_or_default(),
                row.barcode.as_deref().unwrap_or_default(),
                row.dmd_code.as_deref().unwrap_or_default(),
            ],
        ));
    }
    for location in locations
        .iter()
        .filter(|row| row.name.to_lowercase().contains(&query.to_lowercase()))
        .take(LIMIT)
    {
        results.push(SearchResult::new(
            "location",
            location.name.clone(),
            label(labels, "location"),
            format!("/households/{slug}/locations/{}", location.id),
            query,
            &[],
        ));
    }
    let matching_person_ids: Vec<_> = matching_people.iter().map(|row| row.id.get()).collect();
    let matching_medicine_ids = medication::Entity::find()
        .in_tenant(household_id)
        .filter(medication::Column::Name.ilike(&term))
        .all(tenant.transaction())
        .await?
        .into_iter()
        .map(|row| row.id)
        .collect::<Vec<_>>();
    let source_match = Condition::any()
        .add(schedule::Column::MedicationId.is_in(matching_medicine_ids.clone()))
        .add(schedule::Column::PersonId.is_in(matching_person_ids.clone()));
    let mut schedules = access::schedule_scope(tenant)
        .filter(source_match)
        .all(tenant.transaction())
        .await?;
    let mut assignments = person_medication::Entity::find()
        .in_tenant(household_id)
        .filter(person_medication::Column::RetiredAt.is_null())
        .filter(
            person_medication::Column::PersonId
                .is_in(people_by_id.keys().copied().collect::<Vec<_>>()),
        )
        .filter(
            Condition::any()
                .add(person_medication::Column::MedicationId.is_in(matching_medicine_ids))
                .add(person_medication::Column::PersonId.is_in(matching_person_ids)),
        )
        .all(tenant.transaction())
        .await?;
    let source_medicine_ids = schedules
        .iter()
        .map(|row| row.medication_id)
        .chain(assignments.iter().map(|row| row.medication_id))
        .collect::<Vec<_>>();
    let source_medicines: HashMap<_, _> = access::medication_scope(tenant)
        .filter(medication::Column::Id.is_in(source_medicine_ids))
        .all(tenant.transaction())
        .await?
        .into_iter()
        .map(|row| (row.id, row))
        .collect();
    let source_order = |medication_id: i64, person_id: i64| {
        (
            source_medicines
                .get(&medication_id)
                .and_then(|row| row.name.as_deref())
                .unwrap_or_default()
                .to_lowercase(),
            people_by_id
                .get(&person_id)
                .map(|row| row.name.to_lowercase())
                .unwrap_or_default(),
        )
    };
    schedules.sort_by_key(|row| source_order(row.medication_id, row.person_id));
    assignments.sort_by_key(|row| source_order(row.medication_id, row.person_id));
    schedules.truncate(LIMIT);
    assignments.truncate(LIMIT);
    for row in schedules {
        let (Some(medicine), Some(person)) = (
            source_medicines.get(&row.medication_id),
            people_by_id.get(&row.person_id),
        ) else {
            continue;
        };
        let name = medication_name(medicine);
        let title = interpolate(
            &labels["global_search"]["result_titles"]["schedule"],
            &[("medication", &name)],
        );
        let frequency = row.frequency.as_deref().unwrap_or_else(|| {
            labels["global_search"]["subtitles"]["no_frequency"]
                .as_str()
                .unwrap_or_default()
        });
        let subtitle = interpolate(
            &labels["global_search"]["subtitles"]["schedule"],
            &[("person", &person.name), ("frequency", frequency)],
        );
        results.push(SearchResult::new(
            "schedule",
            title,
            subtitle,
            format!(
                "/households/{slug}/people/{}#schedule_{}",
                row.person_id, row.id
            ),
            query,
            &[&person.name],
        ));
    }
    for row in assignments {
        let (Some(medicine), Some(person)) = (
            source_medicines.get(&row.medication_id),
            people_by_id.get(&row.person_id),
        ) else {
            continue;
        };
        let title = medication_name(medicine);
        let subtitle = interpolate(
            &labels["global_search"]["subtitles"]["person_medication"],
            &[("person", &person.name)],
        );
        results.push(SearchResult::new(
            "person_medication",
            title,
            subtitle,
            format!(
                "/households/{slug}/people/{}#person_medication_{}",
                row.person_id, row.id
            ),
            query,
            &[&person.name],
        ));
    }
    Ok(results)
}

fn medication_name(row: &medication::Model) -> String {
    row.friendly_name
        .as_deref()
        .filter(|value| !value.is_empty())
        .or(row.name.as_deref())
        .unwrap_or("Medication")
        .to_owned()
}

fn label(labels: &Value, kind: &str) -> String {
    labels["global_search"]["types"][kind]
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

fn interpolate(template: &Value, values: &[(&str, &str)]) -> String {
    let mut output = template.as_str().unwrap_or_default().to_owned();
    for (key, value) in values {
        output = output.replace(&format!("%{{{key}}}"), value);
    }
    output
}

fn escape_pattern(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn kind_order(kind: &str) -> usize {
    match kind {
        "command" => 0,
        "person" => 1,
        "medication" => 2,
        "person_medication" => 3,
        "schedule" => 4,
        "location" => 5,
        _ => 6,
    }
}
