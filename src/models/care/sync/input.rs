use super::*;

pub struct Operation {
    pub resource_type: String,
    pub action: String,
    pub id: Option<String>,
    pub if_match: Option<String>,
    pub attributes: Map<String, Value>,
}

fn invalid() -> OperationError {
    OperationError::Validation {
        details: json!({"code":"unprocessable_content","message":"Batch is invalid"}),
    }
}

pub fn parse(body: &Value) -> Result<Vec<Operation>, OperationError> {
    let outer = body
        .as_object()
        .filter(|outer| outer.len() == 1)
        .ok_or_else(invalid)?;
    let batch = outer
        .get("batch")
        .and_then(Value::as_object)
        .filter(|batch| batch.len() == 1)
        .ok_or_else(invalid)?;
    let operations = batch
        .get("operations")
        .and_then(Value::as_array)
        .filter(|operations| !operations.is_empty())
        .ok_or_else(invalid)?;
    operations.iter().map(|value| {
        let row = value.as_object().ok_or_else(invalid)?;
        if row.keys().any(|key| !matches!(key.as_str(), "resource_type" | "action" | "id" | "if_match" | "attributes")) {
            return Err(invalid());
        }
        let resource_type = row.get("resource_type").and_then(Value::as_str).ok_or_else(invalid)?;
        let action = row.get("action").and_then(Value::as_str).ok_or_else(invalid)?;
        let actions: &[&str] = match resource_type {
            "medication_take" => &["create"],
            "medication_dose_occurrence" => &["create", "update"],
            "medication_pause_period" => &["create", "close"],
            "medication" => &["create", "update", "delete", "adjust_inventory", "mark_as_ordered", "mark_as_received", "remove_stock"],
            "medication_dosage_option" | "person" => &["create", "update"],
            "health_event" | "location" => &["create", "update", "delete"],
            "medication_review_prompt" => &["update"],
            "schedule" => &["create", "update", "delete", "pause", "resume"],
            "person_medication" => &["create", "update", "delete", "pause", "resume", "reorder"],
            _ => &[],
        };
        if !actions.contains(&action) {
            return Err(OperationError::Validation { details: json!({"code":"sync_operation_unsupported","message":"Operation is not supported offline"}) });
        }
        let id = match row.get("id") {
            None => None,
            Some(Value::String(id)) if crate::models::care::doses::valid_identifier(id) => Some(id.clone()),
            _ => return Err(invalid()),
        };
        if action != "create" && id.is_none() { return Err(invalid()); }
        let if_match = match row.get("if_match") {
            None => None,
            Some(Value::String(value)) => Some(value.clone()),
            _ => return Err(invalid()),
        };
        let attributes = match row.get("attributes") {
            None => Map::new(),
            Some(Value::Object(value)) => value.clone(),
            _ => return Err(invalid()),
        };
        Ok(Operation { resource_type: resource_type.into(), action: action.into(), id, if_match, attributes })
    }).collect()
}
