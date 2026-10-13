use crate::models::errors::OperationError;
use serde_json::Value;
use std::sync::Arc;

fn resolve_key(settings: Option<&Value>) -> Option<Arc<[u8]>> {
    let key = settings?
        .get("dose_occurrences")?
        .get("signing_key")?
        .as_str()?;
    (key.len() >= 32).then(|| Arc::from(key.as_bytes()))
}

pub(crate) fn configured_signing_key(
    settings: Option<&Value>,
) -> Result<Arc<[u8]>, OperationError> {
    resolve_key(settings).ok_or(OperationError::Unavailable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn occurrence_signing_key_is_explicit_and_validated() {
        assert!(resolve_key(None).is_none());
        assert!(resolve_key(Some(&json!({"dose_occurrences":{"signing_key":42}}))).is_none());
        assert!(resolve_key(Some(&json!({"dose_occurrences":{"signing_key":"short"}}))).is_none());
        let configured = "synthetic-configured-occurrence-key-32";
        let settings = json!({"dose_occurrences":{"signing_key":configured}});
        assert_eq!(
            resolve_key(Some(&settings)).unwrap().as_ref(),
            configured.as_bytes()
        );
    }
}
