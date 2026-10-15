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
    #[must_use]
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
    #[doc = "# Errors\nThe profile request fails, is rejected, or cannot be decoded."]
    pub async fn me(&self, household_id: &str) -> Result<Value, ApiError> {
        self.get_data(&format!("/api/v1/households/{household_id}/me"))
            .await
    }

    #[doc = "# Errors\nThe resource request fails, is rejected, or cannot be decoded."]
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
}
