use serde_json::Value;

pub(crate) fn node<'a>(settings: Option<&'a Value>, path: &[&str]) -> Option<&'a Value> {
    let mut current = settings?;
    for key in path {
        current = current.get(*key)?;
        if current.is_null() {
            return None;
        }
    }
    Some(current)
}

pub(crate) fn text<'a>(settings: Option<&'a Value>, path: &[&str]) -> Option<&'a str> {
    node(settings, path)?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(crate) fn truthy(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(value) => Some(*value),
        Value::String(value) => {
            let value = value.trim().to_ascii_lowercase();
            ["1", "true", "yes", "on"]
                .contains(&value.as_str())
                .then_some(true)
                .or_else(|| {
                    ["0", "f", "false", "no", "off"]
                        .contains(&value.as_str())
                        .then_some(false)
                })
        }
        _ => None,
    }
}

pub(crate) fn flag(settings: Option<&Value>, path: &[&str]) -> Option<bool> {
    truthy(node(settings, path)?)
}

pub(crate) fn integer(settings: Option<&Value>, path: &[&str]) -> Option<i64> {
    match node(settings, path)? {
        Value::Number(number) => number.as_i64(),
        Value::String(value) => value.trim().parse().ok(),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PublicOriginError;

impl std::fmt::Display for PublicOriginError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Invalid configured public origin")
    }
}

impl std::error::Error for PublicOriginError {}

impl From<url::ParseError> for PublicOriginError {
    fn from(_: url::ParseError) -> Self {
        Self
    }
}

pub(crate) fn public_origin(ctx: &loco_rs::app::AppContext) -> Result<url::Url, PublicOriginError> {
    public_origin_from_settings(
        ctx.config.settings.as_ref(),
        &ctx.config.server.full_url(),
        &ctx.environment,
    )
}

fn public_origin_from_settings(
    settings: Option<&Value>,
    fallback: &str,
    environment: &loco_rs::environment::Environment,
) -> Result<url::Url, PublicOriginError> {
    let configured = match node(settings, &["public_origin"]) {
        Some(Value::String(value)) if value.trim().is_empty() => None,
        Some(Value::String(value)) => Some(value.trim()),
        Some(_) => return Err(PublicOriginError),
        None => None,
    };
    let origin = url::Url::parse(configured.unwrap_or(fallback))?;
    if !matches!(origin.scheme(), "http" | "https")
        || origin.host_str().is_none()
        || !origin.username().is_empty()
        || origin.password().is_some()
        || origin.path() != "/"
        || origin.query().is_some()
        || origin.fragment().is_some()
        || (configured.is_some()
            && matches!(environment, loco_rs::environment::Environment::Production)
            && origin.scheme() != "https")
    {
        return Err(PublicOriginError);
    }
    Ok(origin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn public_origin_keeps_proxy_ports_separate_from_binding() {
        for (configured, expected) in [
            ("https://mt-dev.localhost", "https://mt-dev.localhost/"),
            (
                "https://mt-dev.localhost:1356",
                "https://mt-dev.localhost:1356/",
            ),
        ] {
            let value = json!({"public_origin": configured});
            let origin = public_origin_from_settings(
                Some(&value),
                "http://localhost:5150",
                &loco_rs::environment::Environment::Development,
            )
            .unwrap();
            assert_eq!(origin.as_str(), expected);
            assert!(!origin.as_str().contains("5150"));
            assert_eq!(
                origin.join("/invitations/accept").unwrap().host_str(),
                Some("mt-dev.localhost")
            );
            assert_eq!(origin.join("/token").unwrap().port(), origin.port());
        }
    }

    #[test]
    fn public_origin_preserves_absent_null_and_empty_fallback() {
        for value in [
            json!({}),
            json!({"public_origin":null}),
            json!({"public_origin":""}),
        ] {
            let origin = public_origin_from_settings(
                Some(&value),
                "http://localhost:5150",
                &loco_rs::environment::Environment::Production,
            )
            .unwrap();
            assert_eq!(origin.as_str(), "http://localhost:5150/");
        }
    }

    #[test]
    fn public_origin_rejects_non_origins_and_insecure_production_override() {
        for configured in [
            "https://user:password@example.test",
            "https://example.test/path",
            "https://example.test?query=1",
            "https://example.test#fragment",
            "file:///tmp/app",
            "invalid",
        ] {
            let value = json!({"public_origin":configured});
            assert!(
                public_origin_from_settings(
                    Some(&value),
                    "http://localhost:5150",
                    &loco_rs::environment::Environment::Development
                )
                .is_err()
            );
        }
        let value = json!({"public_origin":"http://localhost:1356"});
        assert!(
            public_origin_from_settings(
                Some(&value),
                "http://localhost:5150",
                &loco_rs::environment::Environment::Production
            )
            .is_err()
        );
        assert!(
            public_origin_from_settings(
                Some(&value),
                "http://localhost:5150",
                &loco_rs::environment::Environment::Development
            )
            .is_ok()
        );
        assert!(
            public_origin_from_settings(
                Some(&json!({"public_origin":true})),
                "http://localhost:5150",
                &loco_rs::environment::Environment::Development
            )
            .is_err()
        );
    }

    fn settings() -> Value {
        json!({
            "section": {
                "name": " value ",
                "empty": "",
                "on": true,
                "off": false,
                "on_text": "yes",
                "off_text": "0",
                "count": 7,
                "count_text": "30",
                "nullish": null,
                "nested": {"deep": "x", "nothing": null}
            }
        })
    }

    #[test]
    fn text_resolves_nested_non_empty_strings() {
        let value = settings();
        assert_eq!(text(Some(&value), &["section", "name"]), Some("value"));
        assert_eq!(text(Some(&value), &["section", "empty"]), None);
        assert_eq!(text(Some(&value), &["section", "missing"]), None);
        assert_eq!(text(Some(&value), &["missing"]), None);
        assert_eq!(text(None, &["section", "name"]), None);
        assert_eq!(
            text(Some(&value), &["section", "nested", "deep"]),
            Some("x")
        );
    }

    #[test]
    fn flag_accepts_bools_and_env_style_strings() {
        let value = settings();
        assert_eq!(flag(Some(&value), &["section", "on"]), Some(true));
        assert_eq!(flag(Some(&value), &["section", "off"]), Some(false));
        assert_eq!(flag(Some(&value), &["section", "on_text"]), Some(true));
        assert_eq!(flag(Some(&value), &["section", "off_text"]), Some(false));
        assert_eq!(flag(Some(&value), &["section", "empty"]), None);
        assert_eq!(flag(Some(&value), &["section", "name"]), None);
        assert_eq!(flag(Some(&value), &["section", "missing"]), None);
    }

    #[test]
    fn integer_accepts_numbers_and_numeric_strings() {
        let value = settings();
        assert_eq!(integer(Some(&value), &["section", "count"]), Some(7));
        assert_eq!(integer(Some(&value), &["section", "count_text"]), Some(30));
        assert_eq!(integer(Some(&value), &["section", "name"]), None);
        assert_eq!(integer(Some(&value), &["section", "missing"]), None);
    }

    #[test]
    fn null_nodes_resolve_as_absent() {
        let value = settings();
        assert_eq!(node(Some(&value), &["section", "nullish"]), None);
        assert_eq!(node(Some(&value), &["section", "nested", "nothing"]), None);
        assert_eq!(text(Some(&value), &["section", "nullish"]), None);
        assert_eq!(flag(Some(&value), &["section", "nullish"]), None);
        assert_eq!(integer(Some(&value), &["section", "nullish"]), None);
    }
}
