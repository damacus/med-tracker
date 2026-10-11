use std::{collections::BTreeMap, sync::LazyLock};

use super::resource::AuthenticationError;

static RAILS_NAMES: LazyLock<BTreeMap<&'static str, &'static str>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("rails_time_zones.json"))
        .expect("Retained Rails timezone mapping must be valid")
});

pub(crate) fn preferred(
    preferences: &serde_json::Value,
) -> Result<chrono_tz::Tz, AuthenticationError> {
    let fallback = std::env::var("TZ").unwrap_or_else(|_| "UTC".into());
    let name = preferences
        .get("time_zone")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or(&fallback);
    RAILS_NAMES
        .get(name)
        .copied()
        .unwrap_or(name)
        .parse()
        .map_err(|_| AuthenticationError::Unavailable)
}

#[cfg(test)]
mod tests {
    #[test]
    fn retained_rails_names_resolve_to_supported_iana_zones() {
        assert_eq!(super::RAILS_NAMES.len(), 152);
        for (name, identifier) in super::RAILS_NAMES.iter() {
            assert!(
                identifier.parse::<chrono_tz::Tz>().is_ok(),
                "Unsupported retained timezone {name}: {identifier}"
            );
        }
    }
}
