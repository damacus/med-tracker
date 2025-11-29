# ADR 0004: OpenTelemetry for Observability

- Status: Accepted
- Date: 2025-11-29

## Context

MedTracker is a healthcare application that requires robust observability for:

- **Operational monitoring**: Understanding application performance and health
- **Debugging**: Tracing requests across the application stack
- **Compliance**: Audit trail visibility for regulatory requirements (DCB0129, DCB0160)
- **Performance optimization**: Identifying bottlenecks in database queries and service calls

The application is deployed using Docker and needs observability that works well in containerized environments.

## Decision

We adopt **OpenTelemetry** as our observability framework for distributed tracing and metrics.

### Implementation

**Gems added:**

- `opentelemetry-sdk` - Core SDK for OpenTelemetry
- `opentelemetry-exporter-otlp` - OTLP protocol exporter
- `opentelemetry-instrumentation-all` - Auto-instrumentation for common libraries

**Auto-instrumented libraries:**

- Rails (controller actions, routes)
- ActiveRecord (database queries with obfuscated SQL)
- ActionPack (HTTP request handling)
- ActionView (template rendering)
- ActiveJob (background job processing)
- ActiveSupport (caching, notifications)
- Net::HTTP (outbound HTTP calls)
- PG (PostgreSQL queries with obfuscation)
- Rack (middleware timing)

### Configuration

Environment variables control OpenTelemetry behavior:

| Variable | Default | Description |
|----------|---------|-------------|
| `OTEL_ENABLED` | `'true'` (production), `'false'` (other) | Enable/disable tracing (string values) |
| `OTEL_SERVICE_NAME` | `medtracker` | Service name in traces |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://localhost:4318/v1/traces` | OTLP collector endpoint |

### Security Considerations

- SQL statements are obfuscated to prevent sensitive data leakage
- OpenTelemetry is disabled by default in non-production environments
- No PHI (Protected Health Information) is included in trace attributes
- Endpoint authentication should be configured at the collector level

## Consequences

### Positive

- **Vendor-neutral**: OpenTelemetry is CNCF-backed and works with many backends (Jaeger, Zipkin, Datadog, Honeycomb, etc.)
- **Standardized**: Uses OTLP protocol, a widely-adopted standard
- **Low overhead**: Sampling and batching minimize performance impact
- **Rich context**: Automatic propagation of trace context across requests
- **Future-proof**: Growing ecosystem with active development

### Negative

- **Learning curve**: Team needs to understand distributed tracing concepts
- **Infrastructure requirement**: Requires an OTLP collector in production
- **Gem dependencies**: Adds several gems to the bundle

### Trade-offs Accepted

- Accept the additional gems for comprehensive observability
- Accept the need for collector infrastructure for full functionality
- Obfuscate SQL to balance debugging capability with data protection

## Related Documents

- `config/initializers/opentelemetry.rb` - Configuration file
- `docs/app_spec.txt` - Application specification (updated with OTEL in tech stack)
