//! Typed telemetry settings read from `settings.telemetry` in
//! `config/*.yaml` (declared via `get_env`; never `std::env` in `src/`).

use serde_json::Value;

const DEFAULT_TIMEOUT_MS: u64 = 10_000;
const DEFAULT_RATIO: f64 = 1.0;

#[derive(Clone, Debug, Default)]
pub struct TelemetrySettings {
    /// `OTEL_EXPORTER_OTLP_ENDPOINT`: base endpoint; `/v1/traces` is appended
    /// when no explicit traces endpoint is set.
    pub otlp_endpoint: String,
    /// `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT`: exact traces endpoint.
    pub otlp_traces_endpoint: String,
    /// `OTEL_EXPORTER_OTLP_METRICS_ENDPOINT`: read now, used by the metrics PR.
    pub otlp_metrics_endpoint: String,
    /// `OTEL_EXPORTER_OTLP_HEADERS` as `k=v,k2=v2`.
    pub otlp_headers: String,
    /// `OTEL_EXPORTER_OTLP_TIMEOUT` in milliseconds.
    pub otlp_timeout_ms: u64,
    /// `MEDTRACKER_OTEL_TRACE_SAMPLE_RATIO`, clamped to 0..=1.
    pub trace_sample_ratio: f64,
    /// `OTEL_SERVICE_INSTANCE_ID` / pod name.
    pub service_instance_id: String,
}

impl TelemetrySettings {
    pub fn from_settings(settings: Option<&Value>) -> Self {
        let telemetry = settings
            .and_then(|settings| settings.get("telemetry"))
            .cloned()
            .unwrap_or(Value::Null);
        let string = |key: &str| {
            telemetry
                .get(key)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_owned()
        };
        let timeout_ms = telemetry
            .get("otlp_timeout_ms")
            .and_then(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok()))
            .unwrap_or(DEFAULT_TIMEOUT_MS);
        let ratio = telemetry
            .get("trace_sample_ratio")
            .and_then(|value| value.as_f64().or_else(|| value.as_str()?.parse().ok()))
            .filter(|ratio| (0.0..=1.0).contains(ratio))
            .unwrap_or(DEFAULT_RATIO);
        Self {
            otlp_endpoint: string("otlp_endpoint"),
            otlp_traces_endpoint: string("otlp_traces_endpoint"),
            otlp_metrics_endpoint: string("otlp_metrics_endpoint"),
            otlp_headers: string("otlp_headers"),
            otlp_timeout_ms: timeout_ms,
            trace_sample_ratio: ratio,
            service_instance_id: string("service_instance_id"),
        }
    }

    /// Effective traces endpoint: the explicit traces endpoint wins;
    /// otherwise the base endpoint gains the `/v1/traces` suffix.
    pub fn traces_endpoint(&self) -> Option<String> {
        if !self.otlp_traces_endpoint.is_empty() {
            return Some(self.otlp_traces_endpoint.clone());
        }
        if self.otlp_endpoint.is_empty() {
            return None;
        }
        Some(format!(
            "{}/v1/traces",
            self.otlp_endpoint.trim_end_matches('/')
        ))
    }

    /// Parsed `k=v` header pairs. Malformed entries are ignored.
    pub fn headers(&self) -> std::collections::HashMap<String, String> {
        self.otlp_headers
            .split(',')
            .filter_map(|pair| {
                let (key, value) = pair.split_once('=')?;
                let key = key.trim();
                (!key.is_empty()).then(|| (key.to_owned(), value.trim().to_owned()))
            })
            .collect()
    }

    /// Host portion of the effective endpoint for the startup line; never
    /// includes headers or credentials.
    pub fn endpoint_host(&self) -> Option<String> {
        let endpoint = self.traces_endpoint()?;
        url::Url::parse(&endpoint)
            .ok()
            .and_then(|url| url.host_str().map(|host| host.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::TelemetrySettings;
    use serde_json::json;

    #[test]
    fn empty_settings_disable_export() {
        for settings in [None, Some(json!({})), Some(json!({"telemetry": {}}))] {
            let parsed = TelemetrySettings::from_settings(settings.as_ref());
            assert!(parsed.traces_endpoint().is_none());
            assert_eq!(parsed.trace_sample_ratio, 1.0);
            assert_eq!(parsed.otlp_timeout_ms, 10_000);
        }
    }

    #[test]
    fn explicit_traces_endpoint_wins_over_base() {
        let parsed = TelemetrySettings::from_settings(Some(&json!({"telemetry": {
            "otlp_endpoint": "http://collector:4318",
            "otlp_traces_endpoint": "http://traces:4318/custom",
        }})));
        assert_eq!(
            parsed.traces_endpoint().as_deref(),
            Some("http://traces:4318/custom")
        );
        assert_eq!(parsed.endpoint_host().as_deref(), Some("traces"));
    }

    #[test]
    fn base_endpoint_gains_v1_traces_suffix() {
        let parsed = TelemetrySettings::from_settings(Some(&json!({"telemetry": {
            "otlp_endpoint": "http://collector:4318/",
        }})));
        assert_eq!(
            parsed.traces_endpoint().as_deref(),
            Some("http://collector:4318/v1/traces")
        );
    }

    #[test]
    fn invalid_ratio_falls_back_to_default() {
        for value in [json!("fast"), json!(2.0), json!(-0.5)] {
            let parsed = TelemetrySettings::from_settings(Some(&json!({"telemetry": {
                "trace_sample_ratio": value,
            }})));
            assert_eq!(parsed.trace_sample_ratio, 1.0);
        }
        let parsed = TelemetrySettings::from_settings(Some(&json!({"telemetry": {
            "trace_sample_ratio": "0.25",
        }})));
        assert_eq!(parsed.trace_sample_ratio, 0.25);
    }

    #[test]
    fn headers_parse_and_skip_malformed_pairs() {
        let parsed = TelemetrySettings::from_settings(Some(&json!({"telemetry": {
            "otlp_headers": "authorization=Bearer x, bad, x-scope=canary",
        }})));
        let headers = parsed.headers();
        assert_eq!(
            headers.get("authorization").map(String::as_str),
            Some("Bearer x")
        );
        assert_eq!(headers.get("x-scope").map(String::as_str), Some("canary"));
        assert_eq!(headers.len(), 2);
    }
}
