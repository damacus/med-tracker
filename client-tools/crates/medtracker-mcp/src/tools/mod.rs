use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
pub struct EmptyParams {}

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
pub struct HouseholdParams {
    pub household_id: String,
}

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
pub struct ExportJobParams {
    pub household_id: String,
    pub job_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
pub struct ResourceListParams {
    pub household_id: String,
    pub kind: ResourceKind,
}

#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    People,
    Locations,
    Medications,
    DosageOptions,
    HealthEvents,
    Schedules,
    PersonMedications,
    MedicationTakes,
}

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
pub struct SyncChangesParams {
    pub household_id: String,
    pub since: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ToolDefinition {
    pub name: &'static str,
    pub description: &'static str,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

pub fn tool_list() -> Vec<ToolDefinition> {
    vec![
        tool::<EmptyParams>(
            "medtracker_capabilities",
            "Read the MedTracker API capability document.",
        ),
        tool::<EmptyParams>(
            "medtracker_households",
            "List households visible to the credential.",
        ),
        tool::<HouseholdParams>(
            "medtracker_me",
            "Read the current user profile for a household.",
        ),
        tool::<ResourceListParams>(
            "medtracker_resource_list",
            "List a supported household resource collection.",
        ),
        tool::<HouseholdParams>(
            "medtracker_portable_export",
            "Submit an encrypted portable export using the server-configured passphrase.",
        ),
        tool::<ExportJobParams>(
            "medtracker_portable_export_status",
            "Check a portable export job.",
        ),
        tool::<ExportJobParams>(
            "medtracker_portable_export_download",
            "Download a ready encrypted portable export.",
        ),
        tool::<HouseholdParams>("medtracker_sync_snapshot", "Read the sync snapshot."),
        tool::<SyncChangesParams>(
            "medtracker_sync_changes",
            "Read sync changes since a cursor.",
        ),
    ]
}

fn tool<T: JsonSchema>(name: &'static str, description: &'static str) -> ToolDefinition {
    ToolDefinition {
        name,
        description,
        input_schema: json!(schema_for!(T)),
    }
}

impl From<ResourceKind> for medtracker_api_client::ResourceKind {
    fn from(value: ResourceKind) -> Self {
        match value {
            ResourceKind::People => Self::People,
            ResourceKind::Locations => Self::Locations,
            ResourceKind::Medications => Self::Medications,
            ResourceKind::DosageOptions => Self::DosageOptions,
            ResourceKind::HealthEvents => Self::HealthEvents,
            ResourceKind::Schedules => Self::Schedules,
            ResourceKind::PersonMedications => Self::PersonMedications,
            ResourceKind::MedicationTakes => Self::MedicationTakes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_schemas_are_typed() {
        let tools = tool_list();
        let resource_tool = tools
            .iter()
            .find(|tool| tool.name == "medtracker_resource_list");

        assert_eq!(tools.len(), 9);
        for name in [
            "medtracker_portable_export_status",
            "medtracker_portable_export_download",
        ] {
            let tool = tools.iter().find(|tool| tool.name == name).unwrap();
            assert_eq!(
                tool.input_schema
                    .pointer("/properties/job_id/type")
                    .and_then(Value::as_str),
                Some("integer")
            );
        }
        assert_eq!(
            resource_tool
                .and_then(|tool| tool.input_schema.pointer("/properties/household_id/type"))
                .and_then(Value::as_str),
            Some("string")
        );
        assert!(
            resource_tool
                .and_then(|tool| tool.input_schema.pointer("/properties/kind"))
                .is_some()
        );
    }
}
