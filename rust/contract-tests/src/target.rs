use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::redirect::Policy;
use std::env;
use std::time::Duration;
use url::{Host, Url};

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

    pub fn get_with_header(
        &self,
        path: &str,
        token: &str,
        name: &'static str,
        value: &str,
    ) -> Response {
        self.authorize(self.client.get(self.url(path)), Some(token))
            .header(name, value)
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

    pub fn get_html_with_header(&self, path: &str, name: &'static str, value: &str) -> Response {
        self.client
            .get(self.url(path))
            .header("Accept", "text/html")
            .header(name, value)
            .send()
            .expect("target must respond")
    }

    pub fn get_html_from_local_client(&self, path: &str, client_ip: &str) -> Response {
        self.require_local_write();
        self.client
            .get(self.url(path))
            .header("Accept", "text/html")
            .header("X-Forwarded-For", client_ip)
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

    pub fn delete_json(&self, path: &str) -> Response {
        self.require_local_write();
        self.client
            .delete(self.url(path))
            .header("Accept", "application/json")
            .send()
            .expect("target must respond")
    }

    pub fn delete_web_json(&self, path: &str, csrf: &str) -> Response {
        self.require_local_write();
        self.client
            .delete(self.url(path))
            .header("Accept", "application/json")
            .header("X-CSRF-Token", csrf)
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

    pub fn post_browser_form(&self, path: &str, fields: &[(String, String)]) -> Response {
        self.require_local_write();
        self.client
            .post(self.url(path))
            .header("Accept", "text/html")
            .header("Origin", self.origin.origin().ascii_serialization())
            .form(fields)
            .send()
            .expect("target must respond")
    }

    pub fn patch_html_form(&self, path: &str, fields: &[(String, String)]) -> Response {
        self.require_local_write();
        self.client
            .patch(self.url(path))
            .header("Accept", "text/html")
            .form(fields)
            .send()
            .expect("target must respond")
    }

    pub fn put_html_form(&self, path: &str, fields: &[(String, String)]) -> Response {
        self.require_local_write();
        self.client
            .put(self.url(path))
            .header("Accept", "text/html")
            .form(fields)
            .send()
            .expect("target must respond")
    }

    pub fn delete_html_form(&self, path: &str, fields: &[(String, String)]) -> Response {
        self.require_local_write();
        self.client
            .delete(self.url(path))
            .header("Accept", "text/html")
            .form(fields)
            .send()
            .expect("target must respond")
    }

    pub fn post_html_form_from_local_client(
        &self,
        path: &str,
        fields: &[(String, String)],
        client_ip: &str,
    ) -> Response {
        self.require_local_write();
        self.client
            .post(self.url(path))
            .header("Accept", "text/html")
            .header("X-Forwarded-For", client_ip)
            .form(fields)
            .send()
            .expect("target must respond")
    }

    pub fn post_html_form_from_client(
        &self,
        path: &str,
        client_ip: &str,
        fields: &[(String, String)],
    ) -> Response {
        self.post_html_form_from_local_client(path, fields, client_ip)
    }

    pub fn web_form_request(
        &self,
        method: &str,
        path: &str,
        accept: &str,
        fields: &[(String, String)],
    ) -> Response {
        self.require_local_write();
        let method = reqwest::Method::from_bytes(method.as_bytes()).expect("HTTP method");
        self.client
            .request(method, self.url(path))
            .header("Accept", accept)
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

    pub fn post_web_json(
        &self,
        path: &str,
        csrf: &str,
        client_ip: Option<&str>,
        body: &serde_json::Value,
    ) -> Response {
        self.require_local_write();
        let request = self
            .client
            .post(self.url(path))
            .header("Accept", "application/json")
            .header("X-CSRF-Token", csrf);
        let request = match client_ip {
            Some(client_ip) => request.header("X-Forwarded-For", client_ip),
            None => request,
        };
        request.json(body).send().expect("target must respond")
    }

    pub fn post_json_from_web_client(
        &self,
        path: &str,
        client_ip: &str,
        body: &serde_json::Value,
    ) -> Response {
        self.require_local_write();
        self.client
            .post(self.url(path))
            .header("Accept", "application/json")
            .header("X-Forwarded-For", client_ip)
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

    pub fn post_raw_json_authorized(&self, path: &str, token: &str, body: &str) -> Response {
        self.require_local_write();
        self.authorize(self.client.post(self.url(path)), Some(token))
            .header("Content-Type", "application/json")
            .body(body.to_owned())
            .send()
            .expect("target must respond")
    }

    pub fn post_json_with_header(
        &self,
        path: &str,
        token: &str,
        name: &'static str,
        value: &str,
        body: &serde_json::Value,
    ) -> Response {
        self.require_local_write();
        self.authorize(self.client.post(self.url(path)), Some(token))
            .header(name, value)
            .json(body)
            .send()
            .expect("target must respond")
    }

    pub fn post_json_with_key(
        &self,
        path: &str,
        token: &str,
        key: &str,
        body: &serde_json::Value,
    ) -> Response {
        self.require_local_write();
        self.authorize(self.client.post(self.url(path)), Some(token))
            .header("Idempotency-Key", key)
            .json(body)
            .send()
            .expect("target must respond")
    }

    pub fn post_json_from_local_client(
        &self,
        path: &str,
        token: &str,
        client_ip: &str,
        key: Option<&str>,
        body: &serde_json::Value,
    ) -> Response {
        self.require_local_write();
        let request = self
            .authorize(self.client.post(self.url(path)), Some(token))
            .header("X-Forwarded-For", client_ip);
        let request = match key {
            Some(key) => request.header("Idempotency-Key", key),
            None => request,
        };
        request.json(body).send().expect("target must respond")
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

    pub fn patch_json_without_auth(&self, path: &str, body: &serde_json::Value) -> Response {
        self.require_local_write();
        self.client
            .patch(self.url(path))
            .header("Accept", "application/json")
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

    pub fn patch_raw_json_if_match(
        &self,
        path: &str,
        token: &str,
        body: &str,
        etag: &str,
    ) -> Response {
        self.require_local_write();
        self.authorize(self.client.patch(self.url(path)), Some(token))
            .header("If-Match", etag)
            .header("Content-Type", "application/json")
            .body(body.to_owned())
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

    pub fn put_raw_json_if_match(
        &self,
        path: &str,
        token: &str,
        body: &str,
        etag: &str,
    ) -> Response {
        self.require_local_write();
        self.authorize(self.client.put(self.url(path)), Some(token))
            .header("If-Match", etag)
            .header("Content-Type", "application/json")
            .body(body.to_owned())
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

    pub fn put_multipart(
        &self,
        path: &str,
        token: &str,
        form: reqwest::blocking::multipart::Form,
    ) -> Response {
        self.require_local_write();
        self.authorize(self.client.put(self.url(path)), Some(token))
            .multipart(form)
            .send()
            .expect("target must respond")
    }

    pub fn put_json_without_auth(&self, path: &str, body: &serde_json::Value) -> Response {
        self.require_local_write();
        self.client
            .put(self.url(path))
            .header("Accept", "application/json")
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
