use reqwest::blocking::{Client, Response};
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
    pub user_id: i64,
    pub household_id: i64,
    pub foreign_household_id: i64,
    pub foreign_email: String,
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
            .no_proxy()
            .build()
            .expect("HTTP client");
        Self { client, origin }
    }

    pub fn get(&self, path: &str, token: Option<&str>) -> Response {
        let url =
            checked_url(&self.origin, path).expect("request path must stay within target origin");
        let request = self.client.get(url).header("Accept", "application/json");
        let request = match token {
            Some(token) => request.bearer_auth(token),
            None => request,
        };
        request.send().expect("target must respond")
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
