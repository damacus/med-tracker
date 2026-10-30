#[derive(Clone, Copy, Debug)]
pub enum CredentialMethod {
    PersonalApiKey,
    ApiSession,
    ApiAppToken,
    OauthGrant,
    BrowserSession,
}

impl CredentialMethod {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::PersonalApiKey => "personal_api_key",
            Self::ApiSession => "api_session",
            Self::ApiAppToken => "api_app_token",
            Self::OauthGrant => "oauth",
            Self::BrowserSession => "browser_session",
        }
    }

    pub(crate) fn reference_prefix(self) -> &'static str {
        match self {
            Self::PersonalApiKey => "personal_api_key",
            Self::ApiSession => "api_session",
            Self::ApiAppToken => "api_app_token",
            Self::OauthGrant => "oauth_grant",
            Self::BrowserSession => "browser_session",
        }
    }
}

#[derive(Clone, Debug)]
pub struct CredentialProvenance {
    pub method: CredentialMethod,
    pub reference: String,
}
