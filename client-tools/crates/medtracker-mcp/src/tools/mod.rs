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
            "Export a portable household data bundle.",
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
            .find(|tool| tool.name == "medtracker_resource_list")
            .unwrap();

        assert_eq!(tools.len(), 7);
        assert_eq!(
            resource_tool.input_schema["properties"]["household_id"]["type"],
            "string"
        );
        assert!(
            resource_tool.input_schema["properties"]
                .get("kind")
                .is_some()
        );
    }
}
