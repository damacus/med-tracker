use serde::Serialize;
use serde_json::Value;

use crate::client::ApiClient;
use crate::error::ApiError;

#[derive(Debug, Clone, Copy)]
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

impl ResourceKind {
    pub fn path_segment(self) -> &'static str {
        match self {
            Self::People => "people",
            Self::Locations => "locations",
            Self::Medications => "medications",
            Self::DosageOptions => "dosage_options",
            Self::HealthEvents => "health_events",
            Self::Schedules => "schedules",
            Self::PersonMedications => "person_medications",
            Self::MedicationTakes => "medication_takes",
        }
    }
}

impl ApiClient {
    pub async fn me(&self, household_id: &str) -> Result<Value, ApiError> {
        self.get_data(&format!("/api/v1/households/{household_id}/me"))
            .await
    }

    pub async fn list_resource(
        &self,
        household_id: &str,
        kind: ResourceKind,
    ) -> Result<Value, ApiError> {
        self.get_data(&format!(
            "/api/v1/households/{}/{}",
            household_id,
            kind.path_segment()
        ))
        .await
    }

    pub async fn create_resource<B: Serialize + ?Sized>(
        &self,
        household_id: &str,
        kind: ResourceKind,
        body: &B,
    ) -> Result<Value, ApiError> {
        self.post_json_data(
            &format!(
                "/api/v1/households/{}/{}",
                household_id,
                kind.path_segment()
            ),
            body,
        )
        .await
    }
}
