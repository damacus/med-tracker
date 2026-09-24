use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::redirect::Policy;
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::Path;
use std::time::Duration;
use url::{Host, Url};

#[derive(Deserialize)]
pub struct Fixture {
    pub access_token: String,
    pub account_id: i64,
    pub primary_email: String,
    pub user_id: i64,
    pub household_id: i64,
    pub household_name: String,
    pub foreign_household_id: i64,
    pub foreign_email: String,
    pub session_id: i64,
    pub revocable_session_id: i64,
    pub revocable_access_token: String,
    pub logout_access_token: String,
    pub expired_access_token: String,
    pub locked_access_token: String,
    pub oauth_client_id: String,
    pub oauth_redirect_uri: String,
    pub user_person_id: i64,
    pub managed_person_id: i64,
    pub managed_person_portable_id: String,
    pub hidden_person_id: i64,
    pub foreign_person_id: i64,
    pub foreign_person_portable_id: String,
    pub foreign_person_name: String,
    pub view_access_token: String,
    pub delegated_access_token: String,
    pub view_owner_access_token: String,
    pub care_access_token: String,
    pub grant_target_membership_id: i64,
    pub primary_location_id: i64,
    pub primary_location_portable_id: String,
    pub historical_location_portable_id: String,
    pub foreign_location_id: i64,
    pub foreign_location_name: String,
    pub managed_medication_id: i64,
    pub managed_medication_portable_id: String,
    pub hidden_medication_id: i64,
    pub foreign_medication_id: i64,
    pub foreign_medication_portable_id: String,
    pub foreign_medication_name: String,
    pub managed_assignment_id: i64,
    pub hidden_assignment_id: i64,
    pub hidden_assignment_portable_id: String,
    pub foreign_assignment_id: i64,
    pub foreign_assignment_portable_id: String,
    pub hidden_pause_period_id: String,
    pub foreign_pause_period_id: String,
    pub managed_schedule_id: i64,
    pub hidden_schedule_id: i64,
    pub foreign_schedule_id: i64,
    pub foreign_dosage_id: i64,
    pub hidden_dosage_id: i64,
    pub hidden_health_event_id: i64,
    pub foreign_health_event_id: i64,
    pub managed_review_prompt_id: i64,
    pub second_review_prompt_id: i64,
    pub low_signal_review_prompt_id: i64,
    pub edit_review_prompt_id: i64,
    pub invalid_review_prompt_id: i64,
    pub hidden_review_prompt_id: i64,
    pub foreign_review_prompt_id: i64,
}

pub struct Target {
    client: Client,
    origin: Url,
}

impl Target {
    pub fn from_env() -> Self {
        let raw = env::var("CONTRACT_BASE_URL").expect("CONTRACT_BASE_URL is required");
        let origin = Url::parse(&raw).expect("CONTRACT_BASE_URL must be a URL");
        assert!(
            matches!(origin.scheme(), "http" | "https"),
            "HTTP(S) target required"
        );
        assert!(
            origin.username().is_empty() && origin.password().is_none(),
            "URL credentials are forbidden"
        );
        assert!(
            origin.path() == "/" && origin.query().is_none() && origin.fragment().is_none(),
            "target must be an origin"
        );
        let local = match origin.host() {
            Some(Host::Domain("localhost")) => true,
            Some(Host::Ipv4(ip)) => ip.is_loopback(),
            Some(Host::Ipv6(ip)) => ip.is_loopback(),
            _ => false,
        };
        if !local {
            let approved = env::var("CONTRACT_APPROVED_ORIGIN").unwrap_or_default();
            assert_eq!(
                origin.as_str().trim_end_matches('/'),
                approved.trim_end_matches('/'),
                "non-local target requires exact CONTRACT_APPROVED_ORIGIN"
            );
            assert_eq!(
                origin.scheme(),
                "https",
                "approved remote targets must use HTTPS"
            );
        }
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(Policy::none())
            .cookie_store(true)
            .no_proxy()
            .build()
            .expect("HTTP client");
        Self { client, origin }
    }

    pub fn get(&self, path: &str, token: Option<&str>) -> Response {
        self.authorize(self.client.get(self.url(path)), token)
            .send()
            .expect("target must respond")
    }

    pub fn get_html(&self, path: &str) -> Response {
        self.client
            .get(self.url(path))
            .header("Accept", "text/html")
            .send()
            .expect("target must respond")
    }

    pub fn get_from_local_client(&self, path: &str, client_ip: &str) -> Response {
        self.require_local_write();
        self.client
            .get(self.url(path))
            .header("Accept", "application/json")
            .header("X-Forwarded-For", client_ip)
            .send()
            .expect("target must respond")
    }

    pub fn delete(&self, path: &str, token: Option<&str>) -> Response {
        self.require_local_write();
        self.authorize(self.client.delete(self.url(path)), token)
            .send()
            .expect("target must respond")
    }

    pub fn post_form(&self, path: &str, fields: &[(&str, &str)]) -> Response {
        self.require_local_write();
        self.client
            .post(self.url(path))
            .header("Accept", "application/json")
            .form(fields)
            .send()
            .expect("target must respond")
    }

    pub fn post_html_form(&self, path: &str, fields: &[(String, String)]) -> Response {
        self.require_local_write();
        self.client
            .post(self.url(path))
            .header("Accept", "text/html")
            .form(fields)
            .send()
            .expect("target must respond")
    }

    pub fn post_json(&self, path: &str, body: &serde_json::Value) -> Response {
        self.require_local_write();
        self.client
            .post(self.url(path))
            .header("Accept", "application/json")
            .json(body)
            .send()
            .expect("target must respond")
    }

    pub fn post_json_authorized(
        &self,
        path: &str,
        token: &str,
        body: &serde_json::Value,
    ) -> Response {
        self.require_local_write();
        self.authorize(self.client.post(self.url(path)), Some(token))
            .json(body)
            .send()
            .expect("target must respond")
    }

    pub fn post_json_if_match(
        &self,
        path: &str,
        token: &str,
        body: &serde_json::Value,
        etag: &str,
    ) -> Response {
        self.require_local_write();
        self.authorize(self.client.post(self.url(path)), Some(token))
            .header("If-Match", etag)
            .json(body)
            .send()
            .expect("target must respond")
    }

    pub fn patch_json(&self, path: &str, token: &str, body: &serde_json::Value) -> Response {
        self.require_local_write();
        self.authorize(self.client.patch(self.url(path)), Some(token))
            .json(body)
            .send()
            .expect("target must respond")
    }

    pub fn patch_json_if_match(
        &self,
        path: &str,
        token: &str,
        body: &serde_json::Value,
        etag: &str,
    ) -> Response {
        self.require_local_write();
        self.authorize(self.client.patch(self.url(path)), Some(token))
            .header("If-Match", etag)
            .json(body)
            .send()
            .expect("target must respond")
    }

    pub fn put_json_if_match(
        &self,
        path: &str,
        token: &str,
        body: &serde_json::Value,
        etag: &str,
    ) -> Response {
        self.require_local_write();
        self.authorize(self.client.put(self.url(path)), Some(token))
            .header("If-Match", etag)
            .json(body)
            .send()
            .expect("target must respond")
    }

    pub fn put_json(&self, path: &str, token: &str, body: &serde_json::Value) -> Response {
        self.require_local_write();
        self.authorize(self.client.put(self.url(path)), Some(token))
            .json(body)
            .send()
            .expect("target must respond")
    }

    pub fn delete_if_match(&self, path: &str, token: &str, etag: &str) -> Response {
        self.require_local_write();
        self.authorize(self.client.delete(self.url(path)), Some(token))
            .header("If-Match", etag)
            .send()
            .expect("target must respond")
    }

    fn url(&self, path: &str) -> Url {
        checked_url(&self.origin, path).expect("request path must stay within target origin")
    }

    fn authorize(&self, request: RequestBuilder, token: Option<&str>) -> RequestBuilder {
        let request = request.header("Accept", "application/json");
        match token {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    }

    fn require_local_write(&self) {
        let local = match self.origin.host() {
            Some(Host::Domain("localhost")) => true,
            Some(Host::Ipv4(ip)) => ip.is_loopback(),
            Some(Host::Ipv6(ip)) => ip.is_loopback(),
            _ => false,
        };
        assert!(local, "contract write requests require a loopback target");
    }
}

fn checked_url(origin: &Url, path: &str) -> Result<Url, String> {
    if !path.starts_with('/') || path.starts_with("//") {
        return Err("request path must start with one slash".to_string());
    }
    let url = origin.join(path).map_err(|error| error.to_string())?;
    if url.origin() != origin.origin() || !url.username().is_empty() || url.password().is_some() {
        return Err("request path changed target origin".to_string());
    }
    Ok(url)
}

pub fn fixture() -> Fixture {
    let path = env::var("CONTRACT_FIXTURE_PATH").expect("CONTRACT_FIXTURE_PATH is required");
    let metadata = fs::metadata(&path).expect("fixture file must exist");
    assert!(metadata.is_file(), "fixture path must be a file");
    let value = fs::read_to_string(Path::new(&path)).expect("read fixture");
    serde_json::from_str(&value).expect("valid fixture JSON")
}

#[cfg(test)]
mod tests {
    use super::checked_url;
    use url::Url;

    #[test]
    fn accepts_a_path_on_the_approved_origin() {
        let origin = Url::parse("http://127.0.0.1:3000").unwrap();
        let url = checked_url(&origin, "/api/v1/capabilities").unwrap();
        assert_eq!(url.as_str(), "http://127.0.0.1:3000/api/v1/capabilities");
    }

    #[test]
    fn rejects_a_scheme_relative_host_before_request() {
        let origin = Url::parse("http://127.0.0.1:3000").unwrap();
        assert!(checked_url(&origin, "//other.example/api/v1/me").is_err());
    }

    #[test]
    fn rejects_an_absolute_url_before_request() {
        let origin = Url::parse("http://127.0.0.1:3000").unwrap();
        assert!(checked_url(&origin, "https://other.example/api/v1/me").is_err());
    }

    #[test]
    fn rejects_a_backslash_authority_before_request() {
        let origin = Url::parse("http://127.0.0.1:3000").unwrap();
        assert!(checked_url(&origin, "/\\other.example/api/v1/me").is_err());
    }
}
