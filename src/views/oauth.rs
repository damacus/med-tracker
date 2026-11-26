use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct Discovery<'a> {
    pub issuer: &'a str,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub revocation_endpoint: String,
    pub response_types_supported: &'static [&'static str],
    pub response_modes_supported: &'static [&'static str],
    pub grant_types_supported: &'static [&'static str],
    pub code_challenge_methods_supported: &'static [&'static str],
    pub token_endpoint_auth_methods_supported: &'static [&'static str],
}

#[derive(Serialize)]
pub(crate) struct ProtocolError {
    pub error: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}
