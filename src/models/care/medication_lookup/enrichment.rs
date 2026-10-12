use super::*;
use crate::models::{care::review_prompts::evidence, entities::review_evidence};

pub(super) struct Context {
    families: HashMap<String, Value>,
    evidence: Option<Vec<review_evidence::Model>>,
}

impl Context {
    pub(super) async fn load(
        tenant: &TenantTransaction,
        entries: &[(Value, String)],
        visible: &[medication::Model],
    ) -> Result<Self, OperationError> {
        let barcodes: Vec<String> = entries
            .iter()
            .filter_map(|(entry, _)| entry["barcode"].as_str())
            .chain(visible.iter().filter_map(|row| row.barcode.as_deref()))
            .flat_map(barcode_candidates)
            .collect();
        let mut families = HashMap::new();
        if !barcodes.is_empty() {
            let rows = tenant.transaction().query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                "SELECT b.gtin,f.code,f.name,g.code AS group_code,g.name AS group_name FROM nhs_dmd_barcodes b JOIN nhs_dmd_amp_trade_families a ON a.amp_code=b.amp_code JOIN nhs_dmd_trade_families f ON f.id=a.trade_family_id LEFT JOIN nhs_dmd_trade_family_groups g ON g.id=f.trade_family_group_id WHERE b.gtin=ANY($1)", [barcodes.into()])).await?;
            for row in rows {
                let code: String = row.try_get("", "code")?;
                let name: String = row.try_get("", "name")?;
                let group: Option<String> = row.try_get("", "group_code")?;
                let group_name: Option<String> = row.try_get("", "group_name")?;
                families.insert(row.try_get("","gtin")?, json!({"trade_family":{"code":code,"name":name},"trade_family_group":group.map(|code|json!({"code":code,"name":group_name}))}));
            }
        }
        let probe = tenant.transaction().begin().await?;
        let evidence = review_evidence::Entity::find()
            .filter(review_evidence::Column::MatchStatus.is_in(["unreviewed", "reviewed_pair"]))
            .order_by_asc(review_evidence::Column::Id)
            .all(&probe)
            .await;
        let evidence = match evidence {
            Ok(rows) => {
                probe.commit().await?;
                Some(rows)
            }
            Err(_) => {
                probe.rollback().await?;
                None
            }
        };
        Ok(Self { families, evidence })
    }

    pub(super) fn available(&self) -> bool {
        self.evidence.is_some()
    }

    fn family(&self, barcode: &str) -> Option<&Value> {
        barcode_candidates(barcode)
            .iter()
            .find_map(|barcode| self.families.get(barcode))
    }

    pub(super) fn apply(
        &self,
        entry: &mut Value,
        visible: &[medication::Model],
        locations: &HashMap<i64, String>,
        slug: &str,
    ) {
        if let Some(family) = self.family(entry["barcode"].as_str().unwrap_or("")) {
            entry["trade_family"] = family["trade_family"].clone();
            entry["trade_family_group"] = family["trade_family_group"].clone();
            entry["related_medications"] = json!(
                visible
                    .iter()
                    .filter(|row| entry["existing_medication"]["id"].as_i64() != Some(row.id))
                    .filter(|row| row
                        .barcode
                        .as_deref()
                        .and_then(|barcode| self.family(barcode))
                        .is_some_and(|related| related["trade_family"]["code"]
                            == family["trade_family"]["code"]))
                    .map(|row| stock_value(row, locations, slug))
                    .collect::<Vec<_>>()
            );
        }
        let Some(records) = &self.evidence else {
            return;
        };
        let name = entry["name"]
            .as_str()
            .filter(|value| !value.is_empty())
            .or_else(|| entry["display"].as_str())
            .unwrap_or("");
        let mut prompts = Vec::new();
        let mut hidden = 0;
        for row in visible {
            let interacting = row
                .friendly_name
                .as_deref()
                .filter(|value| !value.is_empty())
                .or(row.name.as_deref())
                .unwrap_or("");
            for matched in evidence::matches(name, interacting, records) {
                if matched.risk == "low" || matched.confidence == "low" {
                    hidden += 1;
                    continue;
                }
                let evidence = matched.evidence;
                let excerpt = if matched.excerpt.chars().count() > 500 {
                    format!(
                        "{}...",
                        matched.excerpt.chars().take(497).collect::<String>()
                    )
                } else {
                    matched.excerpt
                };
                prompts.push(json!({"evidence_record_id":evidence.id,"risk_level":matched.risk,"risk_level_label":label(&matched.risk),"match_confidence":matched.confidence,"match_confidence_label":label(&matched.confidence),"matched_term":matched.matched_term,"match_type":matched.match_type,"source_instruction":matched.instruction,"match_reason":matched.reason,"interacting_medication_name":interacting,"description":"Public medicine-label evidence suggests this combination may be worth reviewing with a pharmacist, nurse, GP, or prescriber.","source_name":evidence.source_name,"source_checked_on":evidence.retrieved_on.to_string(),"source_version":evidence.source_version,"source_effective_on":evidence.source_effective_on.map(|date|date.to_string()),"source_url":evidence.source_url,"evidence_text":excerpt}));
            }
        }
        entry["review_prompts"] = json!(prompts);
        entry["review_prompt_filter"] = json!({"hidden_count":hidden});
    }
}

fn label(value: &str) -> String {
    match value {
        "high" => "High".into(),
        "moderate" => "Moderate".into(),
        "low" => "Low".into(),
        "unknown" => "Unknown - unclassified".into(),
        value => value.into(),
    }
}
