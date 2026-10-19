use serde_json::Value;

use crate::client::ApiClient;
use crate::error::ApiError;
use crate::models::{InitialGuardianDeclaration, PortableImportMode, PortableImportRequest};

impl ApiClient {
    #[doc = "# Errors\nThe export request fails, is rejected, or cannot be decoded."]
    pub async fn portable_export(&self, household_id: &str) -> Result<Value, ApiError> {
        self.get_data(&format!(
            "/api/v1/households/{household_id}/portable_export"
        ))
        .await
    }

    #[doc = "# Errors\nThe import request fails, is rejected, or cannot be decoded."]
    pub async fn portable_import(
        &self,
        household_id: &str,
        data: Value,
        passphrase: Option<String>,
        mode: PortableImportMode,
    ) -> Result<Value, ApiError> {
        self.portable_import_with_declarations(household_id, data, passphrase, mode, Vec::new())
            .await
    }

    #[doc = "# Errors\nThe import request or fresh dependent declarations fail validation, or the response cannot be decoded."]
    pub async fn portable_import_with_declarations(
        &self,
        household_id: &str,
        data: Value,
        passphrase: Option<String>,
        mode: PortableImportMode,
        initial_guardian_declarations: Vec<InitialGuardianDeclaration>,
    ) -> Result<Value, ApiError> {
        let endpoint = match mode {
            PortableImportMode::DryRun => {
                format!("/api/v1/households/{household_id}/portable_imports/dry_run")
            }
            PortableImportMode::Apply => {
                format!("/api/v1/households/{household_id}/portable_imports")
            }
        };
        let request = PortableImportRequest {
            data,
            mode,
            passphrase,
            initial_guardian_declarations,
        };

        self.post_data_with_header(
            &endpoint,
            &request,
            (
                "x-medtracker-portable-passphrase",
                request.passphrase.as_deref(),
            ),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    #[tokio::test]
    async fn guardian_import_forwards_only_explicit_external_declarations()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/households/72001/portable_imports/dry_run"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"data":{"applied":false}})),
            )
            .mount(&server)
            .await;
        let client = ApiClient::new(server.uri(), Some("synthetic-token".to_owned()))?;
        let bundle = json!({"format":"synthetic-encrypted-bundle"});
        client
            .portable_import_with_declarations(
                "72001",
                bundle.clone(),
                Some("synthetic portable password".to_owned()),
                PortableImportMode::DryRun,
                vec![InitialGuardianDeclaration {
                    person_portable_id: "synthetic-dependent".to_owned(),
                    accepted: true,
                }],
            )
            .await?;
        let requests = server
            .received_requests()
            .await
            .ok_or("Expected captured import request")?;
        assert_eq!(requests.len(), 1);
        let request = requests
            .first()
            .ok_or("Expected explicit declaration request")?;
        let body: Value = serde_json::from_slice(&request.body)?;
        assert!(
            body == json!({"bundle":bundle,"initial_guardian_declarations":[
            {"person_portable_id":"synthetic-dependent","accepted":true}]})
        );
        assert!(
            request
                .headers
                .get("x-medtracker-portable-passphrase")
                .and_then(|value| value.to_str().ok())
                == Some("synthetic portable password")
        );
        Ok(())
    }

    #[tokio::test]
    async fn portable_import_sends_bundle_and_passphrase_header()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/households/72001/portable_imports"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "data": {"applied":true}
            })))
            .mount(&server)
            .await;
        let client = ApiClient::new(server.uri(), Some("synthetic-token".to_owned()))?;
        let bundle = json!({"format":"synthetic-encrypted-bundle"});
        client
            .portable_import(
                "72001",
                bundle.clone(),
                Some("synthetic portable password".to_owned()),
                PortableImportMode::Apply,
            )
            .await?;
        let requests = server
            .received_requests()
            .await
            .ok_or("Expected captured request")?;
        assert_eq!(requests.len(), 1);
        let request = requests.first().ok_or("Expected import request")?;
        let body: Value = serde_json::from_slice(&request.body)?;
        let bundle_envelope = body == json!({"bundle":bundle});
        let passphrase_header = request
            .headers
            .get("x-medtracker-portable-passphrase")
            .and_then(|value| value.to_str().ok())
            == Some("synthetic portable password");
        let body_secret_absent = body.get("passphrase").is_none();
        assert!(
            bundle_envelope && passphrase_header && body_secret_absent,
            "Import request contract: bundle_envelope={bundle_envelope}, passphrase_header={passphrase_header}, body_secret_absent={body_secret_absent}"
        );
        Ok(())
    }
}
