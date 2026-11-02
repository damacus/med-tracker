//! Trace identifier helpers: W3C `traceparent` parsing and local ID
//! generation used when telemetry export is disabled.

use opentelemetry::{
    trace::TraceContextExt,
    Context,
    trace::{SpanContext, SpanId, TraceFlags, TraceId, TraceState},
};

/// Parses a W3C `traceparent` header value
/// (`version-trace_id-span_id-flags`) into a remote parent [`Context`].
/// Returns `None` for missing or malformed values.
pub fn remote_parent(header: Option<&str>) -> Option<Context> {
    let value = header?.trim();
    let mut parts = value.split('-');
    let version = parts.next()?;
    if version.len() != 2 || version == "ff" {
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
    context.is_valid().then(|| Context::new().with_remote_span_context(context))
}

/// Extracts just the trace ID from a `traceparent` value, for log
/// correlation when the SDK is disabled.
pub fn trace_id_from(header: Option<&str>) -> Option<String> {
    let value = header?;
    let trace = value.split('-').nth(1)?;
    (trace.len() == 32
        && trace != "00000000000000000000000000000000"
        && trace.chars().all(|c| c.is_ascii_hexdigit()))
    .then(|| trace.to_ascii_lowercase())
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
            trace_id_from(Some("00-4BF92F3577B34DA6A3CE929D0E0E4736-00f067aa0ba902b7-00")),
            Some("4bf92f3577b34da6a3ce929d0e0e4736".to_owned())
        );
        assert!(trace_id_from(Some("bad")).is_none());
    }

    #[test]
    fn generated_ids_have_w3c_shape() {
        let trace = new_trace_id();
        assert_eq!(trace.len(), 32);
        assert!(trace.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert_eq!(new_span_id().len(), 16);
    }
}
