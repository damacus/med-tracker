use crate::models::{
    care::review_prompts,
    entities::{household, review_evidence},
    errors::OperationError,
};
use chrono::{NaiveDate, Utc};
use loco_rs::{
    app::AppContext,
    bgworker::BackgroundWorker,
    task::{Task, TaskInfo, Vars},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, DbBackend, EntityTrait,
    IntoActiveModel, QueryFilter, QuerySelect, Set, Statement, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

#[derive(Deserialize)]
struct Manifest {
    selection: Vec<String>,
    targeted_selection: Vec<Target>,
}

#[derive(Deserialize)]
struct Target {
    term: String,
    interaction_targets: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RefreshRun {
    pub run_id: i64,
}

pub struct RefreshWorker {
    ctx: AppContext,
}

#[async_trait::async_trait]
impl BackgroundWorker<RefreshRun> for RefreshWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    async fn perform(&self, args: RefreshRun) -> loco_rs::Result<()> {
        crate::models::canary::work(&self.ctx).await?;
        run(&self.ctx, args.run_id)
            .await
            .map_err(|_| loco_rs::Error::string("Medication review evidence refresh failed"))
    }
}

pub struct RefreshTask;

#[async_trait::async_trait]
impl Task for RefreshTask {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "medication-review-evidence-refresh".into(),
            detail: "Refresh public medicine-label evidence for medication reviews".into(),
        }
    }

    async fn run(&self, ctx: &AppContext, _vars: &Vars) -> loco_rs::Result<()> {
        crate::models::canary::work(ctx).await?;
        let row = ctx.db.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "INSERT INTO medication_review_evidence_refresh_runs (status,label_count,created_count,updated_count,unchanged_count,missing_count,change_summary,created_at,updated_at) VALUES (0,0,0,0,0,0,'{}'::jsonb,timezone('UTC',clock_timestamp()),timezone('UTC',clock_timestamp())) RETURNING id"
        )).await?.ok_or_else(|| loco_rs::Error::string("Refresh run could not be queued"))?;
        let run_id: i64 = row.try_get("", "id")?;
        if RefreshWorker::perform_later(ctx, RefreshRun { run_id })
            .await
            .is_err()
        {
            fail(&ctx.db, run_id, "The refresh could not be queued")
                .await
                .map_err(|_| loco_rs::Error::string("Refresh run failure could not be recorded"))?;
            return Err(loco_rs::Error::string(
                "Medication review evidence refresh could not be queued",
            ));
        }
        Ok(())
    }
}

async fn run(ctx: &AppContext, run_id: i64) -> Result<(), OperationError> {
    let Some(lease) = claim_run(ctx, run_id).await? else {
        return Ok(());
    };
    let result = async {
        let snapshot = fetch_snapshot(ctx).await?;
        import_claimed_snapshot(ctx, &lease, run_id, &snapshot, Utc::now().date_naive()).await
    }
    .await;
    if let Err(error) = &result {
        let message = if matches!(error, OperationError::Unavailable) {
            "The provider response or database was unavailable"
        } else {
            "The evidence refresh could not complete"
        };
        fail(&lease, run_id, message).await?;
    }
    lease.commit().await?;
    result
}

async fn claim_run(
    ctx: &AppContext,
    run_id: i64,
) -> Result<Option<DatabaseTransaction>, OperationError> {
    let lease = ctx.db.begin().await?;
    let locked = lease.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT pg_try_advisory_xact_lock(hashtextextended('medication-review-refresh', $1::bigint)) AS locked", [run_id.into()]
    )).await?.ok_or(OperationError::Unavailable)?.try_get::<bool>("", "locked")?;
    if !locked {
        lease.rollback().await?;
        return Err(OperationError::Unavailable);
    }
    let claimed = ctx.db.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE medication_review_evidence_refresh_runs SET status=1,started_at=COALESCE(started_at,timezone('UTC',clock_timestamp())),completed_at=NULL,error_message=NULL,updated_at=timezone('UTC',clock_timestamp()) WHERE id=$1 AND status IN (0,1,3) RETURNING id", [run_id.into()]
    )).await?;
    if claimed.is_some() {
        return Ok(Some(lease));
    }
    let existing = ctx
        .db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT status FROM medication_review_evidence_refresh_runs WHERE id=$1",
            [run_id.into()],
        ))
        .await?
        .ok_or(OperationError::NotFound)?;
    let status: i32 = existing.try_get("", "status")?;
    lease.rollback().await?;
    if status == 2 {
        Ok(None)
    } else {
        Err(OperationError::Unavailable)
    }
}

async fn fail(
    connection: &impl ConnectionTrait,
    run_id: i64,
    message: &str,
) -> Result<(), OperationError> {
    connection.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE medication_review_evidence_refresh_runs SET status=3,error_message=$2,completed_at=timezone('UTC',clock_timestamp()),updated_at=timezone('UTC',clock_timestamp()) WHERE id=$1 AND status<>2",
        [run_id.into(), message.into()],
    )).await?;
    Ok(())
}

async fn fetch_snapshot(ctx: &AppContext) -> Result<Value, OperationError> {
    let fixture = matches!(ctx.environment, loco_rs::environment::Environment::Test)
        .then(|| {
            ctx.config
                .settings
                .as_ref()
                .and_then(|settings| settings.get("review_refresh_fixture"))
        })
        .flatten();
    let manifest: Manifest = match fixture.and_then(|value| value.get("manifest")) {
        Some(value) => {
            serde_json::from_value(value.clone()).map_err(|_| OperationError::Unavailable)?
        }
        None => serde_yaml_ng::from_str(include_str!(
            "../../../../assets/catalogue/medication_review_labels.yml"
        ))
        .map_err(|_| OperationError::Unavailable)?,
    };
    let endpoint = match fixture {
        Some(value) => value
            .get("endpoint")
            .and_then(Value::as_str)
            .ok_or(OperationError::Unavailable)?,
        None => "https://api.fda.gov/drug/label.json",
    }
    .to_owned();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(25))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| OperationError::Unavailable)?;
    let entries = manifest
        .selection
        .into_iter()
        .map(|term| (term, Vec::new()))
        .chain(
            manifest
                .targeted_selection
                .into_iter()
                .map(|entry| (entry.term, entry.interaction_targets)),
        )
        .collect::<Vec<_>>();
    let mut labels = Vec::with_capacity(entries.len());
    let mut last_updated = None;
    for group in entries.chunks(8) {
        let mut tasks = tokio::task::JoinSet::new();
        for (position, (term, targets)) in group.iter().enumerate() {
            let client = client.clone();
            let term = term.clone();
            let targets = targets.clone();
            let endpoint = endpoint.clone();
            tasks.spawn(async move {
                fetch_label_from(&client, &endpoint, &term, &targets)
                    .await
                    .map(|result| (position, result))
            });
        }
        let mut results = Vec::with_capacity(group.len());
        while let Some(result) = tasks.join_next().await {
            let result = result.map_err(|_| OperationError::Unavailable)??;
            results.push(result);
        }
        results.sort_by_key(|(position, _)| *position);
        for (_, (label, updated)) in results {
            last_updated.get_or_insert(updated);
            labels.push(label);
        }
    }
    let updated = last_updated.ok_or(OperationError::Unavailable)?;
    Ok(json!({"openfda_last_updated":updated,"labels":labels}))
}

async fn fetch_label_from(
    client: &reqwest::Client,
    endpoint: &str,
    term: &str,
    targets: &[String],
) -> Result<(Value, String), OperationError> {
    for field in ["generic_name", "substance_name"] {
        for exact in [true, false] {
            let base = if exact {
                format!("openfda.{field}.exact:\"{}\"", term.to_ascii_uppercase())
            } else {
                format!("openfda.{field}:\"{term}\"")
            };
            let mut search = format!("{base} AND _exists_:drug_interactions");
            for target in targets {
                search.push_str(&format!(" AND drug_interactions:\"{target}\""));
            }
            let mut url = url::Url::parse(endpoint).map_err(|_| OperationError::Unavailable)?;
            url.query_pairs_mut()
                .append_pair("search", &search)
                .append_pair("limit", "20")
                .append_pair("sort", "effective_time:desc");
            let response = client
                .get(url)
                .send()
                .await
                .map_err(|_| OperationError::Unavailable)?;
            if response.status() == reqwest::StatusCode::NOT_FOUND {
                continue;
            }
            if !response.status().is_success()
                || response
                    .content_length()
                    .is_some_and(|length| length > 2_000_000)
            {
                return Err(OperationError::Unavailable);
            }
            let bytes = response
                .bytes()
                .await
                .map_err(|_| OperationError::Unavailable)?;
            if bytes.len() > 2_000_000 {
                return Err(OperationError::Unavailable);
            }
            let body: Value =
                serde_json::from_slice(&bytes).map_err(|_| OperationError::Unavailable)?;
            let results = body
                .get("results")
                .and_then(Value::as_array)
                .filter(|results| !results.is_empty())
                .ok_or(OperationError::Unavailable)?;
            let mut label = select_label(results, term).clone();
            label["selection_term"] = json!(term);
            let updated = body
                .get("meta")
                .and_then(|meta| meta.get("last_updated"))
                .and_then(Value::as_str)
                .ok_or(OperationError::Unavailable)?
                .to_owned();
            return Ok((label, updated));
        }
    }
    Err(OperationError::Unavailable)
}

fn select_label<'a>(results: &'a [Value], term: &str) -> &'a Value {
    let normalized_term = normalize(term);
    let candidates = results
        .iter()
        .filter(|label| human_label(label))
        .collect::<Vec<_>>();
    let matching = candidates
        .iter()
        .copied()
        .filter(|label| {
            substances(label)
                .iter()
                .any(|value| normalize(value).contains(&normalized_term))
        })
        .collect::<Vec<_>>();
    matching
        .iter()
        .copied()
        .min_by_key(|label| substances(label).len())
        .or_else(|| {
            candidates
                .iter()
                .copied()
                .min_by_key(|label| substances(label).len())
        })
        .unwrap_or(&results[0])
}

fn normalize(value: &str) -> String {
    let mut bracketed = false;
    value
        .chars()
        .map(|character| match character {
            '[' => {
                bracketed = true;
                ' '
            }
            ']' => {
                bracketed = false;
                ' '
            }
            _ if bracketed => ' ',
            _ if character.is_alphanumeric() => character.to_ascii_lowercase(),
            _ => ' ',
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn substances(label: &Value) -> Vec<&str> {
    label
        .get("openfda")
        .and_then(|openfda| openfda.get("substance_name"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

fn human_label(label: &Value) -> bool {
    label
        .get("openfda")
        .and_then(|openfda| openfda.get("product_type"))
        .and_then(Value::as_array)
        .is_some_and(|kinds| {
            kinds.iter().any(|kind| {
                kind.as_str()
                    .is_some_and(|value| value.starts_with("HUMAN "))
            })
        })
}

pub(super) fn source_label(
    label: &Value,
    retrieved_on: NaiveDate,
) -> Result<Value, OperationError> {
    let id = label
        .get("set_id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or(OperationError::Unavailable)?;
    let version = label
        .get("version")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or(OperationError::Unavailable)?;
    let effective = label
        .get("effective_time")
        .and_then(Value::as_str)
        .and_then(|value| NaiveDate::parse_from_str(value, "%Y%m%d").ok())
        .ok_or(OperationError::Unavailable)?;
    let sections = label
        .get("drug_interactions")
        .and_then(Value::as_array)
        .filter(|values| !values.is_empty())
        .ok_or(OperationError::Unavailable)?;
    let evidence = sections
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .ok_or(OperationError::Unavailable)
        })
        .collect::<Result<Vec<_>, _>>()?
        .join("\n\n");
    let openfda = label.get("openfda").ok_or(OperationError::Unavailable)?;
    let first = |field: &str| {
        openfda
            .get(field)
            .and_then(Value::as_array)
            .and_then(|values| values.first())
            .and_then(Value::as_str)
    };
    let product = first("brand_name")
        .or_else(|| first("generic_name"))
        .unwrap_or("Unnamed product");
    let selection = label
        .get("selection_term")
        .and_then(Value::as_str)
        .or_else(|| first("substance_name"))
        .unwrap_or("");
    let raw_substances = substances(label);
    let terms = raw_substances
        .iter()
        .copied()
        .filter(|value| normalize(value).contains(&normalize(selection)))
        .collect::<Vec<_>>();
    let mut candidates = vec![normalize(selection)];
    candidates.extend(terms.iter().map(|value| normalize(value)));
    if raw_substances.len() == 1 {
        candidates.extend(
            openfda
                .get("generic_name")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(normalize),
        );
    }
    let mut seen = HashSet::new();
    candidates.retain(|value| !value.is_empty() && seen.insert(value.clone()));
    let classes = if raw_substances.len() == 1 {
        openfda
            .get("pharm_class_epc")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(normalize)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    Ok(
        serde_json::json!({"source_name":"openFDA / DailyMed SPL","source_record_id":id,
        "source_url":format!("https://dailymed.nlm.nih.gov/dailymed/drugInfo.cfm?setid={id}"),
        "source_version":version,"source_effective_on":effective.to_string(),"retrieved_on":retrieved_on.to_string(),
        "product_name":product,"label_section":"Drug Interactions","evidence_text":evidence,
        "active_ingredient":terms.first().copied().or_else(|| first("substance_name")).map(str::to_owned),
        "candidate_terms":candidates,"pharmacologic_classes":classes}),
    )
}

pub async fn import_snapshot(
    ctx: &AppContext,
    run_id: i64,
    snapshot: &Value,
    retrieved_on: NaiveDate,
) -> Result<(), OperationError> {
    let Some(lease) = claim_run(ctx, run_id).await? else {
        return Ok(());
    };
    let result = import_claimed_snapshot(ctx, &lease, run_id, snapshot, retrieved_on).await;
    if result.is_err() {
        fail(&lease, run_id, "The evidence refresh could not complete").await?;
    }
    lease.commit().await?;
    result
}

async fn import_claimed_snapshot(
    ctx: &AppContext,
    lease: &DatabaseTransaction,
    run_id: i64,
    snapshot: &Value,
    retrieved_on: NaiveDate,
) -> Result<(), OperationError> {
    let source_last_updated = snapshot
        .get("openfda_last_updated")
        .and_then(Value::as_str)
        .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
        .ok_or(OperationError::Unavailable)?;
    let labels = snapshot
        .get("labels")
        .and_then(Value::as_array)
        .filter(|labels| !labels.is_empty())
        .ok_or(OperationError::Unavailable)?;
    let mut attributes = HashMap::new();
    for label in labels {
        let row = source_label(label, retrieved_on)?;
        let id = row["source_record_id"]
            .as_str()
            .ok_or(OperationError::Unavailable)?
            .to_owned();
        attributes.insert(id, row);
    }
    let transaction = ctx.db.begin().await?;
    transaction
        .execute_unprepared(
            "LOCK TABLE medication_review_evidence_records IN SHARE ROW EXCLUSIVE MODE",
        )
        .await?;
    let current = review_evidence::Entity::find()
        .filter(review_evidence::Column::SourceName.eq("openFDA / DailyMed SPL"))
        .all(&transaction)
        .await?;
    let mut existing: HashMap<String, review_evidence::Model> = current
        .into_iter()
        .map(|row| (row.source_record_id.clone(), row))
        .collect();
    let mut created: i32 = 0;
    let mut updated: i32 = 0;
    let mut unchanged: i32 = 0;
    let mut changes = Vec::new();
    let mut ids: Vec<_> = attributes.into_iter().collect();
    ids.sort_by(|left, right| left.0.cmp(&right.0));
    for (id, values) in ids {
        let effective =
            NaiveDate::parse_from_str(values["source_effective_on"].as_str().unwrap(), "%Y-%m-%d")
                .map_err(|_| OperationError::Unavailable)?;
        let source_version = values["source_version"].as_str().unwrap().to_owned();
        let source_url = values["source_url"].as_str().unwrap().to_owned();
        let product_name = values["product_name"].as_str().unwrap().to_owned();
        let evidence_text = values["evidence_text"].as_str().unwrap().to_owned();
        let candidate_terms = values["candidate_terms"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let classes = values["pharmacologic_classes"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let ingredient = values["active_ingredient"].as_str().map(str::to_owned);
        if let Some(previous) = existing.remove(&id) {
            let material = previous.source_version.as_deref() != Some(source_version.as_str())
                || previous.source_effective_on != Some(effective)
                || previous.source_url != source_url
                || previous.product_name != product_name
                || previous.evidence_text != evidence_text
                || previous.active_ingredient != ingredient
                || (previous.match_status != "reviewed_pair"
                    && (previous.candidate_terms != candidate_terms
                        || previous.pharmacologic_classes != classes
                        || !previous.interacting_terms.is_empty()
                        || previous.risk_level != "unknown"
                        || previous.match_confidence != "unknown"
                        || previous.match_status != "unreviewed"));
            let mut row = previous.clone().into_active_model();
            row.source_version = Set(Some(source_version.clone()));
            row.source_effective_on = Set(Some(effective));
            row.source_url = Set(source_url);
            row.product_name = Set(product_name);
            row.evidence_text = Set(evidence_text);
            row.retrieved_on = Set(retrieved_on);
            row.active_ingredient = Set(ingredient);
            if previous.match_status != "reviewed_pair" {
                row.candidate_terms = Set(candidate_terms);
                row.pharmacologic_classes = Set(classes);
                row.interacting_terms = Set(Vec::new());
                row.risk_level = Set("unknown".to_owned());
                row.match_confidence = Set("unknown".to_owned());
                row.match_status = Set("unreviewed".to_owned());
            }
            row.updated_at = Set(Utc::now().naive_utc());
            row.update(&transaction).await?;
            if material {
                updated += 1;
                changes.push(json!({"type":"updated","source_record_id":id,"from_version":previous.source_version,"to_version":source_version,"from_effective_on":previous.source_effective_on,"to_effective_on":effective}));
            } else {
                unchanged += 1;
            }
        } else {
            let now = Utc::now().naive_utc();
            review_evidence::ActiveModel {
                source_name: Set("openFDA / DailyMed SPL".to_owned()),
                source_record_id: Set(id.clone()),
                source_url: Set(source_url),
                source_version: Set(Some(source_version.clone())),
                source_effective_on: Set(Some(effective)),
                retrieved_on: Set(retrieved_on),
                product_name: Set(product_name),
                active_ingredient: Set(ingredient),
                label_section: Set("Drug Interactions".to_owned()),
                evidence_text: Set(evidence_text),
                candidate_terms: Set(candidate_terms),
                pharmacologic_classes: Set(classes),
                interacting_terms: Set(Vec::new()),
                risk_level: Set("unknown".to_owned()),
                match_confidence: Set("unknown".to_owned()),
                match_status: Set("unreviewed".to_owned()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&transaction)
            .await?;
            created += 1;
            changes.push(json!({"type":"created","source_record_id":id,"to_version":source_version,"to_effective_on":effective}));
        }
    }
    let missing = existing.len();
    for (id, previous) in existing {
        changes.push(json!({"type":"missing","source_record_id":id,"from_version":previous.source_version,"from_effective_on":previous.source_effective_on}));
    }
    transaction.commit().await?;
    let households = household::Entity::find().all(&ctx.db).await?;
    for household in households {
        if household.status != "active" || household.lifecycle_state != "active" {
            continue;
        }
        let tenant =
            crate::models::care::browser_push::scheduler::household_transaction(ctx, household.id)
                .await?;
        household::Entity::find_by_id(household.id)
            .lock_exclusive()
            .one(&tenant)
            .await?;
        review_prompts::reconcile_household(&tenant, household.id).await?;
        tenant.commit().await?;
    }
    lease.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE medication_review_evidence_refresh_runs SET status=2,source_last_updated=$2,label_count=$3,created_count=$4,updated_count=$5,unchanged_count=$6,missing_count=$7,change_summary=$8::jsonb,completed_at=timezone('UTC',clock_timestamp()),error_message=NULL,updated_at=timezone('UTC',clock_timestamp()) WHERE id=$1",
        [run_id.into(), source_last_updated.into(), (labels.len() as i32).into(), created.into(), updated.into(), unchanged.into(), (missing as i32).into(), json!({"changes":changes}).to_string().into()],
    )).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Json, Router, http::StatusCode, routing::get};
    use serde_json::json;

    #[test]
    fn maps_snapshot_labels_and_rejects_malformed_evidence() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 13).unwrap();
        let label = json!({"set_id":"synthetic-id","version":"2","effective_time":"20261001",
            "drug_interactions":["First.","Second."],"openfda":{"brand_name":["Synthetic medicine"]}});
        let result = source_label(&label, date).unwrap();
        assert_eq!(result["source_record_id"], "synthetic-id");
        assert_eq!(result["evidence_text"], "First.\n\nSecond.");
        assert_eq!(result["source_effective_on"], "2026-10-01");
        assert!(source_label(&json!({"set_id":"synthetic-id","version":"2","effective_time":"invalid","drug_interactions":[],"openfda":{}}), date).is_err());
    }

    #[test]
    fn chooses_a_human_substance_match_with_the_fewest_ingredients() {
        let results = json!([
            {"set_id":"animal","openfda":{"product_type":["ANIMAL DRUG"],"substance_name":["ibuprofen"]}},
            {"set_id":"many","openfda":{"product_type":["HUMAN PRESCRIPTION DRUG"],"substance_name":["ibuprofen","caffeine"]}},
            {"set_id":"one","openfda":{"product_type":["HUMAN OTC DRUG"],"substance_name":["IBUPROFEN [active]"]}},
            {"set_id":"other","openfda":{"product_type":["HUMAN OTC DRUG"],"substance_name":["acetaminophen"]}}
        ]);
        assert_eq!(
            select_label(results.as_array().unwrap(), "ibuprofen")["set_id"],
            "one"
        );
    }

    #[tokio::test]
    async fn provider_responses_are_selected_and_fail_closed_from_local_fixtures() {
        let app = Router::new()
            .route("/valid", get(|| async {
                Json(json!({"meta":{"last_updated":"2026-10-01"},"results":[
                    {"set_id":"selected","version":"2","effective_time":"20261001","drug_interactions":["Synthetic warning"],"openfda":{"product_type":["HUMAN OTC DRUG"],"substance_name":["ibuprofen"]}}
                ]}))
            }))
            .route("/malformed", get(|| async { Json(json!({"meta":{},"results":[]})) }))
            .route("/unavailable", get(|| async { StatusCode::SERVICE_UNAVAILABLE }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let client = reqwest::Client::new();
        let (selected, updated) =
            fetch_label_from(&client, &format!("{origin}/valid"), "ibuprofen", &[])
                .await
                .unwrap();
        assert_eq!(selected["set_id"], "selected");
        assert_eq!(updated, "2026-10-01");
        assert!(
            fetch_label_from(&client, &format!("{origin}/malformed"), "ibuprofen", &[])
                .await
                .is_err()
        );
        assert!(
            fetch_label_from(&client, &format!("{origin}/unavailable"), "ibuprofen", &[])
                .await
                .is_err()
        );
        server.abort();
    }
}
