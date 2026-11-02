//! Generic telemetry attribute key rule shared by log records and span export.

/// Maximum length in bytes for a string attribute value.
pub const VALUE_CAP: usize = 256;

/// Allowed attribute key families: a fixed set of OpenTelemetry
/// semantic-convention prefixes plus any `medtracker.*` key. `request_id`,
/// `trace_id` and `span_id` are reserved framework correlation fields emitted
/// by the request span and log envelope.
const ALLOWED: &[&str] = &[
    "event.name",
    "error.type",
    "exception.",
    "http.",
    "url.",
    "uri.",
    "server.",
    "client.",
    "network.",
    "user_agent.",
    "db.",
    "messaging.",
    "code.",
    "process.",
    "service.",
    "deployment.",
    "telemetry.sdk.",
    "host.",
    "cloud.",
    "container.",
    "k8s.",
    "otel.",
    "request_id",
    "trace_id",
    "span_id",
    "medtracker.",
];

/// Returns true when `key` conforms to the generic telemetry key rule.
pub fn allowed(key: &str) -> bool {
    ALLOWED.iter().any(|prefix| key.starts_with(prefix))
}

/// Truncates `value` to at most [`VALUE_CAP`] bytes on a UTF-8 boundary.
pub fn cap(value: &str) -> &str {
    if value.len() <= VALUE_CAP {
        return value;
    }
    let mut end = VALUE_CAP;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

/// Filters `fields` into (kept, dropped names) using the key rule. String
/// values are capped by the caller's visitor.
pub fn partition<K: AsRef<str>, V>(
    fields: impl IntoIterator<Item = (K, V)>,
) -> (Vec<(K, V)>, Vec<K>) {
    let mut kept = Vec::new();
    let mut dropped = Vec::new();
    for (key, value) in fields {
        if allowed(key.as_ref()) {
            kept.push((key, value));
        } else {
            dropped.push(key);
        }
    }
    (kept, dropped)
}

/// The span-name pattern admitted by the exporter wrapper. Names that do not
/// match are replaced so dynamic text never reaches the backend.
pub fn safe_span_name(name: &str) -> &str {
    let valid = !name.is_empty()
        && name.len() <= 128
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '/' | '.' | '-' | ':'));
    if valid { name } else { "medtracker.redacted" }
}

#[cfg(test)]
mod tests {
    use super::{allowed, cap, partition, safe_span_name, VALUE_CAP};

    #[test]
    fn key_rule_admits_semconv_and_medtracker_keys() {
        for key in [
            "event.name",
            "http.request.method",
            "http.route",
            "http.response.status_code",
            "error.type",
            "db.system.name",
            "messaging.operation.type",
            "code.file.path",
            "medtracker.duration_ms",
            "medtracker.telemetry.dropped_keys",
            "request_id",
            "service.name",
            "deployment.environment.name",
        ] {
            assert!(allowed(key), "{key}");
        }
        for key in ["category", "error_trace_id", "error", "password", "method", "route", "status", "duration_ms"] {
            assert!(!allowed(key), "{key}");
        }
    }

    #[test]
    fn cap_truncates_to_256_bytes_on_utf8_boundary() {
        let ascii = "a".repeat(VALUE_CAP + 10);
        assert_eq!(cap(&ascii).len(), VALUE_CAP);
        let unicode = "é".repeat(VALUE_CAP);
        let capped = cap(&unicode);
        assert!(capped.len() <= VALUE_CAP);
        assert_eq!(capped.len(), VALUE_CAP);
    }

    #[test]
    fn partition_lists_dropped_key_names() {
        let fields = vec![
            ("http.route", "/health"),
            ("secret_key", "x"),
            ("medtracker.outcome", "success"),
        ];
        let (kept, dropped) = partition(fields);
        assert_eq!(kept.len(), 2);
        assert_eq!(dropped, vec!["secret_key"]);
    }

    #[test]
    fn unsafe_span_names_are_replaced() {
        assert_eq!(safe_span_name("GET /health"), "GET /health");
        assert_eq!(safe_span_name("task nhs_dmd_reconcile"), "task nhs_dmd_reconcile");
        assert_eq!(safe_span_name("GET /Clinical\x1bSentinel"), "medtracker.redacted");
        assert_eq!(safe_span_name(""), "medtracker.redacted");
        assert_eq!(safe_span_name(&"x".repeat(200)), "medtracker.redacted");
    }
}
