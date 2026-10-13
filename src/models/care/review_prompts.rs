use loco_rs::model::TenantQueryExt;
pub(crate) mod edit;
pub(crate) mod evidence;
pub mod refresh;

use crate::models::{
    access::{self, TenantTransaction},
    care::{doses, sync},
    entities::{medication, person, person_medication, review_evidence, review_prompt, schedule},
    errors::OperationError,
};
use chrono::{NaiveDate, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseTransaction, DbBackend,
    EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, Statement,
};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

fn display_name(record: &medication::Model) -> String {
    record
        .friendly_name
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .or(record.name.as_deref())
        .unwrap_or("")
        .to_owned()
}

pub fn value(record: &review_prompt::Model) -> Value {
    json!({
        "id": record.id.to_string(),
        "person_id": record.person_id.to_string(),
        "primary_medication_id": record.primary_medication_id.to_string(),
        "interacting_medication_id": record.interacting_medication_id.to_string(),
        "evidence_record_id": record.evidence_record_id.to_string(),
        "primary_medication_name": record.primary_medication_name,
        "interacting_medication_name": record.interacting_medication_name,
        "evidence_source_name": record.evidence_source_name,
        "evidence_source_url": record.evidence_source_url,
        "evidence_source_version": record.evidence_source_version,
        "matched_term": record.matched_term,
        "match_type": record.match_type,
        "source_instruction": record.source_instruction,
        "match_reason": record.match_reason,
        "evidence_text": record.evidence_text,
        "etag": crate::models::care::sync::review_prompts::tag(record),
        "evidence_source_checked_on": record.evidence_source_checked_on.to_string(),
        "evidence_source_effective_on": record.evidence_source_effective_on.to_string(),
        "risk_level": record.risk_level,
        "match_confidence": record.match_confidence,
        "status": record.status,
        "practitioner_name": record.practitioner_name,
        "practitioner_role": record.practitioner_role,
        "review_note": record.review_note,
        "reviewed_on": record.reviewed_on.map(|date| date.to_string()),
        "reviewed_by_membership_id": record.reviewed_by_membership_id.map(|id| id.to_string()),
        "updated_at": record.updated_at.and_utc().to_rfc3339(),
    })
}

#[derive(serde::Deserialize, Default)]
pub struct ListQuery {
    pub page: Option<String>,
    pub per_page: Option<String>,
    pub review_status: Option<String>,
    pub priority: Option<String>,
    pub show_hidden: Option<String>,
    pub updated: Option<String>,
}

pub struct Filters {
    page: u64,
    size: u64,
    review_status: String,
    priority: String,
    show_hidden: bool,
}

impl Filters {
    pub fn parse(query: ListQuery) -> Option<Self> {
        let page = query
            .page
            .map_or(Some(1), |value| value.parse::<u64>().ok())?;
        let size = query
            .per_page
            .map_or(Some(20), |value| value.parse::<u64>().ok())?;
        let review_status = query
            .review_status
            .unwrap_or_else(|| "needs_review".to_owned());
        let priority = query.priority.unwrap_or_else(|| "all".to_owned());
        let show_hidden = query.show_hidden.unwrap_or_else(|| "0".to_owned());
        if page == 0
            || size == 0
            || size > 100
            || page
                .checked_sub(1)
                .and_then(|page| page.checked_mul(size))
                .is_none_or(|offset| offset > i64::MAX as u64)
            || !matches!(review_status.as_str(), "needs_review" | "reviewed" | "all")
            || !matches!(
                priority.as_str(),
                "all" | "discuss_soon" | "ask_when_convenient" | "low_confidence"
            )
            || !matches!(show_hidden.as_str(), "0" | "1")
        {
            return None;
        }
        Some(Self {
            page,
            size,
            review_status,
            priority,
            show_hidden: show_hidden == "1",
        })
    }
}

fn today_in_app_zone() -> NaiveDate {
    Utc::now().with_timezone(&doses::app_zone()).date_naive()
}

pub async fn index(
    tenant: &TenantTransaction,
    query: Option<ListQuery>,
) -> Result<Value, OperationError> {
    access::recheck(tenant).await?;
    let context = sync::review_prompts::AuthContext { tenant };
    if !sync::review_prompts::actor_adult(tenant.transaction(), &context).await? {
        return Err(OperationError::Forbidden);
    }
    let Some(filters) = query.and_then(Filters::parse) else {
        return Err(OperationError::Validation {
            details: json!({"error":"Review filters are invalid"}),
        });
    };
    sync::lock(tenant).await?;
    if !sync::review_prompts::actor_adult(tenant.transaction(), &context).await? {
        return Err(OperationError::Forbidden);
    }
    let people = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(person::Column::Id.in_subquery(access::granted_people(tenant.membership())))
        .all(tenant.transaction())
        .await?;
    let ids: Vec<i64> = people.iter().map(|row| row.id).collect();
    refresh_many(tenant, &ids, today_in_app_zone()).await?;
    let mut query = review_prompt::Entity::find()
        .filter(review_prompt::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(
            review_prompt::Column::PersonId
                .in_subquery(access::granted_people(tenant.membership())),
        );
    let hidden_count = query
        .clone()
        .filter(review_prompt::Column::Status.eq("hidden_low_signal"))
        .count(tenant.transaction())
        .await?;
    if !filters.show_hidden {
        query = query.filter(review_prompt::Column::Status.ne("hidden_low_signal"));
    }
    let review_counts = json!({
        "needs_review": query.clone().filter(review_prompt::Column::Status.eq("needs_review")).count(tenant.transaction()).await? + if filters.show_hidden { hidden_count } else { 0 },
        "reviewed": query.clone().filter(review_prompt::Column::Status.is_not_in(["needs_review", "hidden_low_signal"])).count(tenant.transaction()).await?,
        "all": query.clone().count(tenant.transaction()).await?,
    });
    query = match filters.review_status.as_str() {
        "needs_review" if filters.show_hidden => {
            query.filter(review_prompt::Column::Status.is_in(["needs_review", "hidden_low_signal"]))
        }
        "needs_review" => query.filter(review_prompt::Column::Status.eq("needs_review")),
        "reviewed" => query
            .filter(review_prompt::Column::Status.is_not_in(["needs_review", "hidden_low_signal"])),
        _ => query,
    };
    query = match filters.priority.as_str() {
        "discuss_soon" => query.filter(review_prompt::Column::RiskLevel.eq("high")),
        "ask_when_convenient" => query.filter(review_prompt::Column::RiskLevel.eq("moderate")),
        "low_confidence" => query.filter(
            Condition::any()
                .add(review_prompt::Column::RiskLevel.is_in(["low", "unknown"]))
                .add(review_prompt::Column::MatchConfidence.is_in(["low", "unknown"])),
        ),
        _ => query,
    };
    let total = query.clone().count(tenant.transaction()).await?;
    let rows = query
        .order_by_asc(review_prompt::Column::PersonId)
        .order_by_asc(review_prompt::Column::CreatedAt)
        .order_by_asc(review_prompt::Column::Id)
        .offset((filters.page - 1) * filters.size)
        .limit(filters.size)
        .all(tenant.transaction())
        .await?;
    Ok(
        json!({"data": rows.iter().map(value).collect::<Vec<_>>(), "meta": {
            "page": filters.page, "per_page": filters.size, "total_count": total,
            "hidden_count": hidden_count, "review_counts": review_counts,
            "active_status": filters.review_status, "active_priority": filters.priority,
            "show_hidden": filters.show_hidden
        }}),
    )
}

pub async fn show(tenant: &TenantTransaction, id: &str) -> Result<(Value, String), OperationError> {
    access::recheck(tenant).await?;
    let context = sync::review_prompts::AuthContext { tenant };
    if !sync::review_prompts::actor_adult(tenant.transaction(), &context).await? {
        return Err(OperationError::Forbidden);
    }
    sync::lock(tenant).await?;
    if !sync::review_prompts::actor_adult(tenant.transaction(), &context).await? {
        return Err(OperationError::Forbidden);
    }
    let Some(record) =
        sync::review_prompts::visible_prompt(tenant.transaction(), &context, id).await?
    else {
        return Err(OperationError::NotFound);
    };
    refresh(tenant, record.person_id, today_in_app_zone()).await?;
    let record = sync::review_prompts::visible_prompt(tenant.transaction(), &context, id)
        .await?
        .ok_or(OperationError::NotFound)?;
    let history = edit::history(tenant, &record).await?;
    Ok((
        json!({"data": value(&record),"history":history}),
        sync::review_prompts::tag(&record),
    ))
}

pub async fn refresh(
    tenant: &TenantTransaction,
    person_id: i64,
    today: NaiveDate,
) -> Result<(), OperationError> {
    refresh_many(tenant, &[person_id], today).await
}

pub async fn refresh_many(
    tenant: &TenantTransaction,
    person_ids: &[i64],
    today: NaiveDate,
) -> Result<(), OperationError> {
    if person_ids.is_empty() {
        return Ok(());
    }
    let db = tenant.transaction();
    let household_id = tenant.scope().household_id;
    reconcile_existing(tenant, person_ids).await?;
    let assignments = person_medication::Entity::find()
        .in_tenant(household_id)
        .filter(person_medication::Column::PersonId.is_in(person_ids.to_vec()))
        .filter(person_medication::Column::Active.eq(true))
        .filter(person_medication::Column::RetiredAt.is_null())
        .all(db)
        .await?;
    let schedules = schedule::Entity::find()
        .in_tenant(household_id)
        .filter(schedule::Column::PersonId.is_in(person_ids.to_vec()))
        .filter(schedule::Column::Active.eq(true))
        .filter(schedule::Column::RetiredAt.is_null())
        .filter(schedule::Column::StartDate.lte(today))
        .filter(schedule::Column::EndDate.gte(today))
        .all(db)
        .await?;
    let all_medication_ids: Vec<i64> = assignments
        .iter()
        .map(|row| row.medication_id)
        .chain(schedules.iter().map(|row| row.medication_id))
        .collect();
    let mut all_medication_ids = all_medication_ids;
    all_medication_ids.sort_unstable();
    all_medication_ids.dedup();
    if all_medication_ids.len() < 2 {
        return Ok(());
    }
    let medications: HashMap<i64, medication::Model> = medication::Entity::find()
        .in_tenant(household_id)
        .filter(medication::Column::Id.is_in(all_medication_ids))
        .all(db)
        .await?
        .into_iter()
        .map(|row| (row.id, row))
        .collect();
    let evidence = review_evidence::Entity::find()
        .filter(review_evidence::Column::MatchStatus.is_in(["unreviewed", "reviewed_pair"]))
        .order_by_asc(review_evidence::Column::Id)
        .all(db)
        .await?;
    let mut existing: HashSet<(i64, i64, i64, i64)> = review_prompt::Entity::find()
        .in_tenant(household_id)
        .filter(review_prompt::Column::PersonId.is_in(person_ids.to_vec()))
        .all(db)
        .await?
        .into_iter()
        .map(|row| {
            (
                row.person_id,
                row.primary_medication_id,
                row.interacting_medication_id,
                row.evidence_record_id,
            )
        })
        .collect();
    for &person_id in person_ids {
        let mut medication_ids: Vec<i64> = assignments
            .iter()
            .filter(|row| row.person_id == person_id)
            .map(|row| row.medication_id)
            .chain(
                schedules
                    .iter()
                    .filter(|row| row.person_id == person_id)
                    .map(|row| row.medication_id),
            )
            .collect();
        medication_ids.sort_unstable();
        medication_ids.dedup();
        for (first_index, first_id) in medication_ids.iter().enumerate() {
            for second_id in medication_ids.iter().skip(first_index + 1) {
                let (Some(first), Some(second)) =
                    (medications.get(first_id), medications.get(second_id))
                else {
                    continue;
                };
                let first_name = display_name(first);
                let second_name = display_name(second);
                for matched in evidence::matches(&first_name, &second_name, &evidence) {
                    let (primary, interacting, primary_name, interacting_name) =
                        if matched.primary_is_first {
                            (first, second, &first_name, &second_name)
                        } else {
                            (second, first, &second_name, &first_name)
                        };
                    if !existing.insert((
                        person_id,
                        primary.id,
                        interacting.id,
                        matched.evidence.id,
                    )) {
                        continue;
                    }
                    let now = Utc::now().naive_utc();
                    let created = review_prompt::ActiveModel {
                        household_id: Set(household_id),
                        person_id: Set(person_id),
                        primary_medication_id: Set(primary.id),
                        interacting_medication_id: Set(interacting.id),
                        evidence_record_id: Set(matched.evidence.id),
                        primary_medication_name: Set(primary_name.clone()),
                        interacting_medication_name: Set(interacting_name.clone()),
                        evidence_source_name: Set(matched.evidence.source_name.clone()),
                        evidence_source_url: Set(matched.evidence.source_url.clone()),
                        evidence_source_checked_on: Set(matched.evidence.retrieved_on),
                        evidence_source_effective_on: Set(matched
                            .evidence
                            .source_effective_on
                            .unwrap_or(matched.evidence.retrieved_on)),
                        evidence_source_version: Set(matched
                            .evidence
                            .source_version
                            .clone()
                            .unwrap_or_else(|| "unknown".to_owned())),
                        evidence_text: Set(matched.excerpt),
                        matched_term: Set(matched.matched_term),
                        match_type: Set(matched.match_type.to_owned()),
                        source_instruction: Set(matched.instruction.to_owned()),
                        match_reason: Set(matched.reason),
                        risk_level: Set(matched.risk.clone()),
                        match_confidence: Set(matched.confidence.clone()),
                        status: Set(if matched.risk == "low" || matched.confidence == "low" {
                            "hidden_low_signal"
                        } else {
                            "needs_review"
                        }
                        .to_owned()),
                        created_at: Set(now),
                        updated_at: Set(now),
                        ..Default::default()
                    }
                    .insert(db)
                    .await?;
                    db.execute_raw(Statement::from_sql_and_values(
                        DbBackend::Postgres,
                        "UPDATE medication_review_prompts p SET evidence_source_fingerprint=md5(jsonb_build_array(e.source_name,e.source_url,e.source_version,e.source_effective_on,e.evidence_text,e.risk_level,e.match_confidence)::text) FROM medication_review_evidence_records e WHERE p.id=$1 AND e.id=p.evidence_record_id",
                        [created.id.into()],
                    )).await?;
                }
            }
        }
    }
    Ok(())
}

async fn reconcile_existing(
    tenant: &TenantTransaction,
    person_ids: &[i64],
) -> Result<(), OperationError> {
    reconcile(
        tenant.transaction(),
        tenant.scope().household_id,
        Some(person_ids),
    )
    .await
}

pub(crate) async fn reconcile_person(
    tenant: &TenantTransaction,
    person_id: i64,
) -> Result<(), OperationError> {
    reconcile_existing(tenant, &[person_id]).await
}

pub(crate) async fn reconcile_household(
    db: &DatabaseTransaction,
    household_id: i64,
) -> Result<(), OperationError> {
    reconcile(db, household_id, None).await
}

async fn reconcile(
    db: &DatabaseTransaction,
    household_id: i64,
    person_ids: Option<&[i64]>,
) -> Result<(), OperationError> {
    let person_filter = person_ids.map_or_else(String::new, |ids| {
        let ids = ids.iter().map(i64::to_string).collect::<Vec<_>>().join(",");
        format!("AND p.person_id IN ({ids})")
    });
    let sql = format!(
        "WITH changed AS (
            SELECT p.id,p.household_id,
                (to_jsonb(p) - 'household_id' - 'created_at' - 'evidence_source_fingerprint') ||
                jsonb_build_object(
                    'id',p.id::text,
                    'person_id',p.person_id::text,
                    'primary_medication_id',p.primary_medication_id::text,
                    'interacting_medication_id',p.interacting_medication_id::text,
                    'evidence_record_id',p.evidence_record_id::text,
                    'reviewed_by_membership_id',p.reviewed_by_membership_id::text,
                    'updated_at',to_char(p.updated_at,'YYYY-MM-DD') || 'T' || to_char(p.updated_at,'HH24:MI:SS.US') || '+00:00',
                    'etag',chr(34) || encode(public.digest(
                        'MedicationReviewPrompt:' || p.id::text || ':' ||
                        floor(extract(epoch from p.updated_at))::bigint::text || '.' ||
                        to_char(p.updated_at,'US'),'sha256'),'hex') || chr(34)
                ) AS snapshot
            FROM medication_review_prompts p
            JOIN medication_review_evidence_records e ON e.id=p.evidence_record_id
            WHERE p.household_id=$1 {person_filter} AND p.evidence_source_fingerprint IS NOT NULL AND
                p.evidence_source_fingerprint IS DISTINCT FROM
                md5(jsonb_build_array(e.source_name,e.source_url,e.source_version,
                    e.source_effective_on,e.evidence_text,e.risk_level,e.match_confidence)::text)
            FOR UPDATE OF p
        ), archived AS (
            INSERT INTO medication_review_histories (household_id,prompt_id,event_type,snapshot)
            SELECT household_id,id,'evidence',snapshot FROM changed RETURNING prompt_id
        ), audited AS (
            INSERT INTO security_audit_events (household_id,event_type,request_id,audit_context,metadata,created_at,updated_at)
            SELECT household_id,'medication_review_prompt.evidence_refreshed','medication-review-refresh',
                '{{}}'::jsonb,jsonb_build_object('prompt_id',id),
                timezone('UTC',clock_timestamp()),timezone('UTC',clock_timestamp()) FROM changed
            RETURNING id
        ) UPDATE medication_review_prompts p SET
            evidence_source_name=e.source_name,evidence_source_url=e.source_url,
            evidence_source_version=COALESCE(e.source_version,'unknown'),
            evidence_source_effective_on=COALESCE(e.source_effective_on,e.retrieved_on),
            evidence_text=e.evidence_text,evidence_source_checked_on=e.retrieved_on,
            evidence_source_fingerprint=md5(jsonb_build_array(e.source_name,e.source_url,
                e.source_version,e.source_effective_on,e.evidence_text,e.risk_level,e.match_confidence)::text),
            risk_level=e.risk_level,match_confidence=e.match_confidence,status='needs_review',
            practitioner_name=NULL,practitioner_role=NULL,reviewed_on=NULL,
            review_note=NULL,reviewed_by_membership_id=NULL,
            updated_at=timezone('UTC',clock_timestamp())
        FROM medication_review_evidence_records e,archived a
        WHERE p.id=a.prompt_id AND e.id=p.evidence_record_id"
    );
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        [household_id.into()],
    ))
    .await?;
    let checked = format!(
        "UPDATE medication_review_prompts p SET evidence_source_checked_on=e.retrieved_on FROM medication_review_evidence_records e WHERE p.household_id=$1 {person_filter} AND e.id=p.evidence_record_id AND p.evidence_source_checked_on IS DISTINCT FROM e.retrieved_on"
    );
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        checked,
        [household_id.into()],
    ))
    .await?;
    Ok(())
}
