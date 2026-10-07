use crate::models::errors::OperationError;
use serde_json::Value;
use std::sync::Arc;

fn resolve_key(override_key: Option<&str>, settings: Option<&Value>) -> Option<Arc<[u8]>> {
    let key = if let Some(value) = override_key {
        value
    } else {
        settings?
            .get("dose_occurrences")?
            .get("signing_key")?
            .as_str()?
    };
    (key.len() >= 32).then(|| Arc::from(key.as_bytes()))
}

pub(crate) fn configured_signing_key(
    settings: Option<&Value>,
) -> Result<Arc<[u8]>, OperationError> {
    let override_key = match std::env::var("AUTH_SESSION_SECRET") {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(_) => return Err(OperationError::Unavailable),
    };
    resolve_key(override_key.as_deref(), settings).ok_or(OperationError::Unavailable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn occurrence_signing_key_is_explicit_and_validated() {
        assert!(resolve_key(None, None).is_none());
        assert!(resolve_key(None, Some(&json!({"dose_occurrences":{"signing_key":42}}))).is_none());
        assert!(
            resolve_key(
                None,
                Some(&json!({"dose_occurrences":{"signing_key":"short"}}))
            )
            .is_none()
        );
        let configured = "synthetic-configured-occurrence-key-32";
        let override_key = "synthetic-override-occurrence-key-32";
        let settings = json!({"dose_occurrences":{"signing_key":configured}});
        assert_eq!(
            resolve_key(None, Some(&settings)).unwrap().as_ref(),
            configured.as_bytes()
        );
        assert_eq!(
            resolve_key(Some(override_key), Some(&settings))
                .unwrap()
                .as_ref(),
            override_key.as_bytes()
        );
        assert!(resolve_key(Some("short"), Some(&settings)).is_none());
    }
}
