//! Generic telemetry attribute key rule shared by log records and span export.

/// Maximum length in bytes for a string attribute value.
pub const VALUE_CAP: usize = 256;

/// Allowed attribute keys: a fixed set of exact OpenTelemetry
/// semantic-convention names plus the `medtracker.*` prefix. Anything else —
/// including free-form families such as `url.full`, `client.address`,
/// `user_agent.original`, `db.query.text`, `exception.message` or
/// `http.request.header.*` — is dropped so private request and clinical data
/// can never reach telemetry. `request_id`, `trace_id` and `span_id` are
/// reserved framework correlation fields emitted by the request span and log
/// envelope; `otel.name`, `otel.kind` and `otel.status_code` are consumed by
/// the export layer itself.
const ALLOWED: &[&str] = &[
    "event.name",
    "error.type",
    "exception.type",
    "http.request.method",
    "http.route",
    "http.response.status_code",
    "url.scheme",
    "server.address",
    "server.port",
    "network.protocol.name",
    "network.protocol.version",
    "db.system.name",
    "db.operation.name",
    "db.collection.name",
    "db.response.status_code",
    "messaging.system",
    "messaging.operation.type",
    "messaging.operation.name",
    "messaging.destination.name",
    "code.function.name",
    "code.file.path",
    "code.line.number",
    "service.name",
    "service.namespace",
    "service.version",
    "service.instance.id",
    "deployment.environment.name",
    "telemetry.sdk.name",
    "telemetry.sdk.language",
    "telemetry.sdk.version",
    "host.name",
    "k8s.pod.name",
    "k8s.namespace.name",
    "otel.name",
    "otel.kind",
    "otel.status_code",
    "request_id",
    "trace_id",
    "span_id",
];

/// Returns true when `key` conforms to the generic telemetry key rule.
pub fn allowed(key: &str) -> bool {
    key.starts_with("medtracker.") || ALLOWED.contains(&key)
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
    use super::{VALUE_CAP, allowed, cap, partition, safe_span_name};

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
        for key in [
            "category",
            "error_trace_id",
            "error",
            "password",
            "method",
            "route",
            "status",
            "duration_ms",
            "url.full",
            "url.query",
            "url.path",
            "client.address",
            "network.peer.address",
            "user_agent.original",
            "db.query.text",
            "exception.message",
            "exception.stacktrace",
            "http.request.header.cookie",
            "http.request.header.authorization",
            "otel.status_description",
            "uri.path",
            "uri.query",
        ] {
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
        assert_eq!(
            safe_span_name("task nhs_dmd_reconcile"),
            "task nhs_dmd_reconcile"
        );
        assert_eq!(
            safe_span_name("GET /Clinical\x1bSentinel"),
            "medtracker.redacted"
        );
        assert_eq!(safe_span_name(""), "medtracker.redacted");
        assert_eq!(safe_span_name(&"x".repeat(200)), "medtracker.redacted");
    }
}
