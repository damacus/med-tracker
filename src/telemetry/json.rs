//! OTel log-data-model JSON layer: one JSON object per line on the writer,
//! no ANSI, span-chain fields merged under event fields.

use std::io::Write;

use opentelemetry::trace::TraceContextExt;
use serde_json::{Map, Value, json};
use tracing::{Event, Subscriber, field::Visit, span};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::{Layer, layer::Context, registry::LookupSpan};

use super::keys;

/// Static resource fields copied into every record's `resource` object.
#[derive(Clone, Debug)]
pub struct Resource {
    pub service_version: String,
    pub environment: String,
    pub instance_id: String,
}

/// Fields captured from a span or event after applying the key rule.
#[derive(Default)]
struct Fields {
    attributes: Vec<(String, Value)>,
    dropped: Vec<String>,
    body: Option<String>,
}

#[derive(Default)]
struct FieldVisitor {
    fields: Fields,
}

impl FieldVisitor {
    fn push(&mut self, name: &str, value: Value) {
        if name == "message" {
            self.fields.body = value
                .as_str()
                .map(|s| keys::cap(s).to_owned())
                .map(Value::String)
                .and_then(|v| v.as_str().map(str::to_owned));
            return;
        }
        if keys::allowed(name) {
            self.fields.attributes.push((name.to_owned(), value));
        } else {
            self.fields.dropped.push(name.to_owned());
        }
    }

    fn debug_value(value: &dyn std::fmt::Debug) -> Value {
        let rendered = format!("{value:?}");
        let trimmed = rendered
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .unwrap_or(&rendered);
        Value::String(keys::cap(trimmed).to_owned())
    }
}

impl Visit for FieldVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.push(field.name(), Value::String(keys::cap(value).to_owned()));
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.push(field.name(), Self::debug_value(value));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.push(field.name(), json!(value));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.push(field.name(), json!(value));
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.push(field.name(), json!(value));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.push(field.name(), json!(value));
    }
}

/// Span extension holding the span's conforming fields and dropped key names.
#[derive(Default)]
struct SpanFields {
    attributes: Vec<(String, Value)>,
    dropped: Vec<String>,
}

/// OTel log-data-model JSON layer.
pub struct JsonLayer<W> {
    writer: W,
    resource: Resource,
    diagnostics: std::sync::Arc<super::export::Diagnostics>,
}

/// Builds the JSON layer over `writer` (stdout in production, a shared buffer
/// in tests).
pub fn layer<W>(
    writer: W,
    resource: Resource,
    diagnostics: std::sync::Arc<super::export::Diagnostics>,
) -> JsonLayer<W>
where
    W: for<'writer> tracing_subscriber::fmt::MakeWriter<'writer> + Send + Sync + 'static,
{
    JsonLayer {
        writer,
        resource,
        diagnostics,
    }
}

fn severity_number(level: &tracing::Level) -> u8 {
    match *level {
        tracing::Level::ERROR => 17,
        tracing::Level::WARN => 13,
        tracing::Level::INFO => 9,
        tracing::Level::DEBUG => 5,
        tracing::Level::TRACE => 1,
    }
}

/// Reserves `trace_id`/`span_id` span fields: they are identifiers, not
/// attributes, so they lift into the envelope rather than `attributes`.
fn split_identifiers(attributes: &mut Vec<(String, Value)>) -> (Option<String>, Option<String>) {
    let mut trace = None;
    let mut span_id = None;
    attributes.retain(|(key, value)| {
        let text = value.as_str().unwrap_or_default();
        match key.as_str() {
            "trace_id" if trace.is_none() => {
                trace = Some(text.to_owned());
                false
            }
            "span_id" if span_id.is_none() => {
                span_id = Some(text.to_owned());
                false
            }
            _ => true,
        }
    });
    (trace, span_id)
}

impl<S, W> Layer<S> for JsonLayer<W>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    W: for<'writer> tracing_subscriber::fmt::MakeWriter<'writer> + Send + Sync + 'static,
{
    fn on_new_span(&self, attributes: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
        let mut visitor = FieldVisitor::default();
        attributes.record(&mut visitor);
        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(SpanFields {
                attributes: visitor.fields.attributes,
                dropped: visitor.fields.dropped,
            });
        }
    }

    fn on_record(&self, id: &span::Id, values: &span::Record<'_>, ctx: Context<'_, S>) {
        let mut visitor = FieldVisitor::default();
        values.record(&mut visitor);
        if let Some(span) = ctx.span(id) {
            let mut extensions = span.extensions_mut();
            let fields = extensions.get_mut::<SpanFields>();
            if let Some(fields) = fields {
                fields.attributes.extend(visitor.fields.attributes);
                fields.dropped.extend(visitor.fields.dropped);
            } else {
                extensions.insert(SpanFields {
                    attributes: visitor.fields.attributes,
                    dropped: visitor.fields.dropped,
                });
            }
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        if event.metadata().target().starts_with("opentelemetry") {
            self.diagnostics.sdk_event(event.metadata().name());
            self.diagnostics.suppress_one();
            return;
        }
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);

        let mut attributes: Vec<(String, Value)> = Vec::new();
        let mut dropped: Vec<String> = Vec::new();
        let mut trace_id = None;
        let mut span_id = None;

        if let Some(scope) = ctx.event_scope(event) {
            for span in scope {
                let extensions = span.extensions();
                if let Some(fields) = extensions.get::<SpanFields>() {
                    let mut inherited = fields.attributes.clone();
                    let (trace, span_ident) = split_identifiers(&mut inherited);
                    if trace.is_some() {
                        trace_id = trace;
                    }
                    if span_ident.is_some() {
                        span_id = span_ident;
                    }
                    attributes.extend(inherited);
                    dropped.extend(fields.dropped.clone());
                }
            }
        }

        // Innermost event fields win over the span chain.
        attributes.extend(visitor.fields.attributes);
        dropped.extend(visitor.fields.dropped);

        // Prefer the live OTel context when the export layer is installed.
        let otel_context = tracing::Span::current().context();
        let otel_span_context = otel_context.span().span_context().clone();
        if otel_span_context.is_valid() {
            trace_id = Some(otel_span_context.trace_id().to_string());
            span_id = Some(otel_span_context.span_id().to_string());
        }

        // Dedup attributes keeping the last (innermost) value per key.
        let mut map = Map::new();
        for (key, value) in attributes {
            map.insert(key, value);
        }
        if !dropped.is_empty() {
            dropped.sort();
            dropped.dedup();
            map.insert(
                "medtracker.telemetry.dropped_keys".to_owned(),
                json!(dropped),
            );
        }

        let mut record = Map::new();
        record.insert(
            "timestamp".to_owned(),
            Value::String(
                chrono::Utc::now()
                    .format("%Y-%m-%dT%H:%M:%S%.6fZ")
                    .to_string(),
            ),
        );
        record.insert(
            "severity_text".to_owned(),
            Value::String(event.metadata().level().to_string()),
        );
        record.insert(
            "severity_number".to_owned(),
            json!(severity_number(event.metadata().level())),
        );
        record.insert(
            "body".to_owned(),
            Value::String(
                visitor
                    .fields
                    .body
                    .unwrap_or_else(|| event.metadata().name().to_owned()),
            ),
        );
        if let Some(trace_id) = trace_id {
            record.insert("trace_id".to_owned(), Value::String(trace_id));
        }
        if let Some(span_id) = span_id {
            record.insert("span_id".to_owned(), Value::String(span_id));
        }
        let mut resource = Map::new();
        resource.insert("service.name".to_owned(), json!("medtracker"));
        resource.insert(
            "service.version".to_owned(),
            json!(self.resource.service_version),
        );
        resource.insert(
            "deployment.environment.name".to_owned(),
            json!(self.resource.environment),
        );
        if !self.resource.instance_id.is_empty() {
            resource.insert(
                "service.instance.id".to_owned(),
                json!(self.resource.instance_id),
            );
        }
        record.insert("resource".to_owned(), Value::Object(resource));
        let mut scope = Map::new();
        scope.insert(
            "name".to_owned(),
            Value::String(event.metadata().target().to_owned()),
        );
        record.insert("scope".to_owned(), Value::Object(scope));
        record.insert("attributes".to_owned(), Value::Object(map));

        let line = Value::Object(record).to_string();
        let mut writer = self.writer.make_writer();
        // Fail-open: a logging failure must never affect the operation.
        let _ = writer.write_all(line.as_bytes());
        let _ = writer.write_all(b"\n");
        let _ = writer.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::{Resource, layer};
    use std::io::Write;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::{layer::SubscriberExt, registry};

    #[derive(Clone, Default)]
    struct Buffer(Arc<Mutex<Vec<u8>>>);

    impl Buffer {
        fn lines(&self) -> Vec<serde_json::Value> {
            let buffer = self.0.lock().expect("log buffer");
            String::from_utf8_lossy(&buffer)
                .lines()
                .map(|line| serde_json::from_str(line).expect("log line is valid JSON"))
                .collect()
        }

        fn raw(&self) -> String {
            String::from_utf8_lossy(&self.0.lock().expect("log buffer")).to_string()
        }
    }

    struct BufferWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for BufferWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().expect("log buffer").extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Buffer {
        type Writer = BufferWriter;

        fn make_writer(&'a self) -> Self::Writer {
            BufferWriter(self.0.clone())
        }
    }

    fn resource() -> Resource {
        Resource {
            service_version: "0.1.0+abcdef0".to_owned(),
            environment: "test".to_owned(),
            instance_id: String::new(),
        }
    }

    fn emit(buffer: &Buffer, event: impl FnOnce()) {
        let subscriber = registry().with(layer(
            buffer.clone(),
            resource(),
            super::super::export::Diagnostics::new(),
        ));
        tracing::subscriber::with_default(subscriber, event);
    }

    #[test]
    fn emits_exact_otel_log_shape_without_ansi() {
        let buffer = Buffer::default();
        emit(&buffer, || {
            tracing::info!(
                event.name = "http.server.request",
                http.request.method = "GET",
                http.route = "/health",
                http.response.status_code = 200_u16,
                medtracker.duration_ms = 0.13,
                "HTTP request completed"
            );
        });
        let raw = buffer.raw();
        assert!(!raw.contains('\x1b'), "ANSI escape present: {raw}");
        assert_eq!(raw.trim_end().lines().count(), 1);
        let record = &buffer.lines()[0];
        for key in [
            "timestamp",
            "severity_text",
            "severity_number",
            "body",
            "resource",
            "scope",
            "attributes",
        ] {
            assert!(record.get(key).is_some(), "missing {key}: {record}");
        }
        assert_eq!(record["severity_text"], "INFO");
        assert_eq!(record["severity_number"], 9);
        assert_eq!(record["body"], "HTTP request completed");
        assert_eq!(record["resource"]["service.name"], "medtracker");
        assert_eq!(record["resource"]["service.version"], "0.1.0+abcdef0");
        assert_eq!(record["resource"]["deployment.environment.name"], "test");
        assert!(record["resource"].get("service.instance.id").is_none());
        assert_eq!(
            record["scope"]["name"],
            "med_tracker::telemetry::json::tests"
        );
        let attributes = &record["attributes"];
        assert_eq!(attributes["event.name"], "http.server.request");
        assert_eq!(attributes["http.request.method"], "GET");
        assert_eq!(attributes["http.route"], "/health");
        assert_eq!(attributes["http.response.status_code"], 200);
        assert!((attributes["medtracker.duration_ms"].as_f64().unwrap() - 0.13).abs() < 1e-9);
        let timestamp = record["timestamp"].as_str().unwrap();
        assert!(
            timestamp.ends_with('Z') && timestamp.contains('.'),
            "{timestamp}"
        );
    }

    #[test]
    fn span_fields_merge_with_innermost_winning() {
        let buffer = Buffer::default();
        emit(&buffer, || {
            let outer =
                tracing::info_span!("outer", http.route = "/outer", medtracker.depth = 1_u64);
            let _outer = outer.enter();
            let inner =
                tracing::info_span!("inner", http.route = "/inner", medtracker.extra = true);
            let _inner = inner.enter();
            tracing::info!(http.route = "/event", "merged");
        });
        eprintln!("RAW: {}", buffer.raw());
        let attributes = &buffer.lines()[0]["attributes"];
        assert_eq!(attributes["http.route"], "/event");
        assert_eq!(attributes["medtracker.depth"], 1);
        assert_eq!(attributes["medtracker.extra"], true, "{}", buffer.raw());
    }

    #[test]
    fn nonconforming_keys_drop_and_report_names() {
        let buffer = Buffer::default();
        emit(&buffer, || {
            tracing::warn!(secret_key = "Sentinel", error.type = "io", "dropped field");
        });
        let record = &buffer.lines()[0];
        assert_eq!(record["severity_number"], 13);
        let attributes = &record["attributes"];
        assert!(attributes.get("secret_key").is_none());
        assert_eq!(attributes["error.type"], "io");
        assert_eq!(
            attributes["medtracker.telemetry.dropped_keys"],
            serde_json::json!(["secret_key"])
        );
        assert!(!buffer.raw().contains("Sentinel"));
    }

    #[test]
    fn string_values_are_capped_at_256_bytes() {
        let buffer = Buffer::default();
        let long = "x".repeat(300);
        emit(&buffer, || {
            tracing::info!(error.type = %long, "capped");
        });
        let attributes = &buffer.lines()[0]["attributes"];
        assert_eq!(attributes["error.type"].as_str().unwrap().len(), 256);
    }

    #[test]
    fn recorded_trace_identifiers_lift_into_the_envelope() {
        let buffer = Buffer::default();
        emit(&buffer, || {
            let span = tracing::info_span!(
                "request",
                trace_id = tracing::field::Empty,
                span_id = tracing::field::Empty
            );
            span.record("trace_id", "4bf92f3577b34da6a3ce929d0e0e4736");
            span.record("span_id", "00f067aa0ba902b7");
            let _entered = span.enter();
            tracing::error!("inside");
        });
        let record = &buffer.lines()[0];
        assert_eq!(record["severity_number"], 17);
        assert_eq!(record["trace_id"], "4bf92f3577b34da6a3ce929d0e0e4736");
        assert_eq!(record["span_id"], "00f067aa0ba902b7");
        assert!(record["attributes"].get("trace_id").is_none());
        assert!(record["attributes"].get("span_id").is_none());
    }
}
