//! Trace identifier helpers: W3C `traceparent` parsing and local ID
//! generation used when telemetry export is disabled.

use opentelemetry::{
    Context,
    trace::TraceContextExt,
    trace::{SpanContext, SpanId, TraceFlags, TraceId, TraceState},
};
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Parses a W3C `traceparent` header value
/// (`version-trace_id-span_id-flags`) into a remote parent [`Context`].
/// Returns `None` for missing or malformed values.
pub fn remote_parent(header: Option<&str>) -> Option<Context> {
    let value = header?.trim();
    let mut parts = value.split('-');
    let version = parts.next()?;
    if version.len() != 2
        || !version.bytes().all(|byte| byte.is_ascii_hexdigit())
        || version == "ff"
    {
        return None;
    }
    let trace = parts.next()?;
    let span = parts.next()?;
    let flags = parts.next()?;
    if parts.next().is_some() || trace.len() != 32 || span.len() != 16 || flags.len() != 2 {
        return None;
    }
    let trace_id = TraceId::from_hex(trace).ok()?;
    let span_id = SpanId::from_hex(span).ok()?;
    let flags = u8::from_str_radix(flags, 16).ok()?;
    let context = SpanContext::new(
        trace_id,
        span_id,
        TraceFlags::new(flags),
        true,
        TraceState::default(),
    );
    context
        .is_valid()
        .then(|| Context::new().with_remote_span_context(context))
}

/// Extracts just the trace ID from a `traceparent` value, for log
/// correlation when the SDK is disabled.
pub fn trace_id_from(header: Option<&str>) -> Option<String> {
    remote_parent(header).map(|context| context.span().span_context().trace_id().to_string())
}

/// Generates a random 32-hex trace ID.
pub fn new_trace_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

/// Generates a random 16-hex span ID.
pub fn new_span_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()[..16].to_owned()
}

#[cfg(test)]
mod tests {
    use super::{new_span_id, new_trace_id, remote_parent, trace_id_from};
    use opentelemetry::trace::TraceContextExt;

    #[test]
    fn valid_traceparent_produces_remote_context() {
        let context = remote_parent(Some(
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-00",
        ))
        .expect("valid traceparent");
        let span_context = context.span().span_context().clone();
        assert_eq!(
            span_context.trace_id().to_string(),
            "4bf92f3577b34da6a3ce929d0e0e4736"
        );
        assert!(span_context.is_remote());
        assert!(!span_context.is_sampled());
    }

    #[test]
    fn malformed_traceparent_is_ignored() {
        for value in [
            None,
            Some(""),
            Some("garbage"),
            Some("ff-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"),
            Some("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01-extra"),
            Some("00-00000000000000000000000000000000-00f067aa0ba902b7-01"),
        ] {
            assert!(remote_parent(value).is_none(), "{value:?}");
        }
    }

    #[test]
    fn trace_id_extraction_matches_parser() {
        assert_eq!(
            trace_id_from(Some(
                "00-4BF92F3577B34DA6A3CE929D0E0E4736-00f067aa0ba902b7-00"
            )),
            Some("4bf92f3577b34da6a3ce929d0e0e4736".to_owned())
        );
        assert!(trace_id_from(Some("bad")).is_none());
    }

    #[test]
    fn generated_ids_have_w3c_shape() {
        let trace = new_trace_id();
        assert_eq!(trace.len(), 32);
        assert!(
            trace
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
        assert_eq!(new_span_id().len(), 16);
    }
}

tokio::task_local! {
    static TRACEPARENT: String;
}

/// Attaches a span to its trace: joins a valid inbound `traceparent` (in
/// either mode) and falls back to generated W3C-format IDs recorded on the
/// span when no valid OTel context exists. Returns the trace ID.
pub fn attach(span: &tracing::Span, inbound: Option<&str>) -> String {
    if let Some(parent) = remote_parent(inbound) {
        let _ = span.set_parent(parent);
    }
    let span_context = span.context();
    let otel_context = span_context.span().span_context().clone();
    if otel_context.is_valid() {
        span.record("trace_id", otel_context.trace_id().to_string().as_str());
        span.record("span_id", otel_context.span_id().to_string().as_str());
        otel_context.trace_id().to_string()
    } else {
        let generated = trace_id_from(inbound).unwrap_or_else(new_trace_id);
        span.record("trace_id", generated.as_str());
        span.record("span_id", new_span_id().as_str());
        generated
    }
}

/// Stores the active traceparent for the current unit of work so
/// `current_traceparent` works even when export is disabled.
pub async fn hold<F: std::future::Future>(traceparent: String, future: F) -> F::Output {
    TRACEPARENT.scope(traceparent, future).await
}

/// Composes an outbound `traceparent` for propagating the active trace to
/// another unit of work. Uses the live OTel context when valid; otherwise
/// joins the same trace ID with a fresh propagation span ID.
pub fn outbound_traceparent(span: &tracing::Span, trace_id: &str) -> String {
    let context = span.context().span().span_context().clone();
    let flags = "01";
    let span_id = if context.is_valid() {
        context.span_id().to_string()
    } else {
        new_span_id()
    };
    format!("00-{trace_id}-{span_id}-{flags}")
}

/// The current spans W3C `traceparent` value, in either export mode.
pub fn current_traceparent() -> Option<String> {
    if let Ok(value) = TRACEPARENT.try_with(|value| value.clone()) {
        return Some(value);
    }
    let span_context = tracing::Span::current().context();
    let context = span_context.span().span_context().clone();
    context.is_valid().then(|| {
        let flags = if context.trace_flags().is_sampled() {
            "01"
        } else {
            "00"
        };
        format!("00-{}-{}-{flags}", context.trace_id(), context.span_id())
    })
}

/// The current request or unit-of-work trace identifier, when available.
pub fn current_trace_id() -> Option<String> {
    trace_id_from(current_traceparent().as_deref())
}

/// Adds the active trace to an existing JSON error without changing its shape.
pub fn enrich_error(body: &mut serde_json::Value) {
    let Some(trace_id) = current_trace_id() else {
        return;
    };
    let Some(object) = body.as_object_mut() else {
        return;
    };
    if let Some(error) = object
        .get_mut("error")
        .and_then(serde_json::Value::as_object_mut)
    {
        error.insert("trace_id".into(), trace_id.into());
    } else {
        object.insert("trace_id".into(), trace_id.into());
    }
}

#[cfg(test)]
mod strictness_tests {
    use super::{attach, remote_parent, trace_id_from};

    const MALFORMED: [&str; 3] = [
        "zz-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
        "ff-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
        "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01-extra",
    ];

    #[test]
    fn malformed_headers_never_echo_their_trace_id() {
        for value in MALFORMED {
            assert!(remote_parent(Some(value)).is_none(), "{value}");
            assert!(trace_id_from(Some(value)).is_none(), "{value}");
        }
    }

    #[test]
    fn attach_generates_fresh_ids_for_malformed_headers() {
        for value in MALFORMED {
            let span = tracing::info_span!(
                "unit",
                trace_id = tracing::field::Empty,
                span_id = tracing::field::Empty
            );
            let trace = attach(&span, Some(value));
            assert_ne!(trace, "4bf92f3577b34da6a3ce929d0e0e4736", "{value}");
            assert_eq!(trace.len(), 32);
            assert!(
                trace.chars().all(|c| c.is_ascii_hexdigit()),
                "{value} -> {trace}"
            );
        }
    }
}
