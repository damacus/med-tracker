# OpenTelemetry Integration Research for MedTracker Rails Application

## Executive Summary

OpenTelemetry (OTel) is an open-source observability framework that provides a single set of APIs, libraries, agents, and instrumentation to capture distributed traces and metrics from applications. This document outlines the research and implementation plan for adding OpenTelemetry to the MedTracker Rails 8.x application.

## What is OpenTelemetry?

OpenTelemetry is a Cloud Native Computing Foundation (CNCF) project that:
- Provides vendor-agnostic instrumentation for distributed tracing, metrics, and logs
- Offers automatic instrumentation for common frameworks and libraries
- Enables observability across microservices and distributed systems
- Supports multiple backend exporters (Jaeger, Zipkin, Prometheus, etc.)

## Benefits for MedTracker

1. **Performance Monitoring**: Track request latency, database queries, and external API calls
2. **Error Detection**: Quickly identify and diagnose errors in production
3. **Dependency Mapping**: Visualize service dependencies and communication patterns
4. **Database Query Analysis**: Monitor and optimize database query performance
5. **User Experience Tracking**: Understand user journey and identify bottlenecks
6. **Production Debugging**: Trace requests through the entire application stack

## Ruby/Rails OpenTelemetry Ecosystem

### Core Gems

1. **opentelemetry-sdk** - Core SDK for OpenTelemetry
2. **opentelemetry-api** - API definitions and interfaces
3. **opentelemetry-exporter-otlp** - OTLP protocol exporter (recommended)
4. **opentelemetry-instrumentation-all** - Meta-gem for all automatic instrumentations

### Rails-Specific Instrumentation

The `opentelemetry-instrumentation-all` gem provides automatic instrumentation for:
- ActionController
- ActionView
- ActiveRecord
- ActiveJob (Solid Queue)
- ActionMailer
- ActiveSupport
- ActiveStorage
- Rack
- Net::HTTP
- ConcurrentRuby

## Implementation

### Configuration

The initializer at `config/initializers/opentelemetry.rb` configures:

```ruby
OpenTelemetry::SDK.configure do |c|
  c.service_name = ENV.fetch('OTEL_SERVICE_NAME', 'med-tracker')
  c.service_version = ENV.fetch('OTEL_SERVICE_VERSION', '1.0.0')

  c.resource = OpenTelemetry::SDK::Resources::Resource.create(
    'deployment.environment' => Rails.env.to_s,
    'service.namespace' => ENV.fetch('OTEL_SERVICE_NAMESPACE', 'default')
  )

  # Auto-instrument all supported libraries (includes OTLP exporter)
  c.use_all
end
```

Note: `c.use_all` automatically configures the OTLP exporter and all supported instrumentations.

### Environment Variables

Configuration is done via standard OpenTelemetry environment variables:

- `OTEL_SERVICE_NAME` - Service name (default: med-tracker)
- `OTEL_SERVICE_VERSION` - Service version (default: 1.0.0)
- `OTEL_SERVICE_NAMESPACE` - Service namespace (default: default)
- `OTEL_EXPORTER_OTLP_ENDPOINT` - OTLP endpoint (default: http://localhost:4318)
- `OTEL_TRACES_EXPORTER` - Set to `none` to disable tracing

### Test Environment

OpenTelemetry is automatically skipped in the test environment to avoid overhead.

## Development Setup with Jaeger

For local development, use Jaeger with Docker:

```bash
docker run -d --name jaeger \
  -e COLLECTOR_OTLP_ENABLED=true \
  -p 16686:16686 \
  -p 4318:4318 \
  jaegertracing/all-in-one:latest
```

Access Jaeger UI at: http://localhost:16686

## Performance Impact

OpenTelemetry is designed to have minimal performance impact:
- Automatic instrumentation adds ~1-2% overhead
- Sampling can further reduce overhead
- Asynchronous export doesn't block request processing
- Can be disabled per-environment

## Security Considerations

1. **Sensitive Data**: Be careful not to log sensitive information in spans
   - Don't include passwords, API keys, or PII in span attributes
   - Use span attribute filtering in production

2. **Network Security**: Secure the connection to the OTel collector
   - Use TLS for production
   - Configure proper authentication

## Backend Compatibility

Works with:
- **Jaeger** - Open-source, self-hosted (recommended for development)
- **Honeycomb** - Commercial, excellent UX
- **New Relic** - Full APM suite
- **Datadog** - Full observability platform
- **Grafana Tempo** - Works with Grafana stack
- **AWS X-Ray** - AWS-native solution
- **Google Cloud Trace** - GCP-native solution

## References

- [OpenTelemetry Ruby Documentation](https://opentelemetry.io/docs/instrumentation/ruby/)
- [OpenTelemetry Ruby GitHub](https://github.com/open-telemetry/opentelemetry-ruby)
- [OpenTelemetry Rails Instrumentation](https://github.com/open-telemetry/opentelemetry-ruby-contrib/tree/main/instrumentation/rails)
