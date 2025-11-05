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

1. **opentelemetry-instrumentation-rails** - Rails framework instrumentation
   - ActionController instrumentation
   - ActionView instrumentation
   - ActiveSupport instrumentation

2. **opentelemetry-instrumentation-active_record** - Database query tracing
   - SQL query tracing
   - Connection pool monitoring

3. **opentelemetry-instrumentation-action_pack** - ActionPack components
4. **opentelemetry-instrumentation-action_view** - View rendering tracing
5. **opentelemetry-instrumentation-active_support** - ActiveSupport cache tracing

### Additional Instrumentation for MedTracker Stack

1. **opentelemetry-instrumentation-rack** - Rack middleware instrumentation
2. **opentelemetry-instrumentation-net_http** - HTTP client instrumentation
3. **opentelemetry-instrumentation-pg** or **opentelemetry-instrumentation-mysql2** - Database driver (for production)

Note: Currently using SQLite3 in development, but these would be useful for production PostgreSQL/MySQL.

## Implementation Approach for Rails 8.x

### 1. Gem Installation

Add to `Gemfile`:
```ruby
# OpenTelemetry for observability and distributed tracing
gem 'opentelemetry-sdk'
gem 'opentelemetry-exporter-otlp'
gem 'opentelemetry-instrumentation-all'
```

The `opentelemetry-instrumentation-all` gem includes:
- Rails framework instrumentation
- ActiveRecord instrumentation
- Rack instrumentation
- Net::HTTP instrumentation
- And many more common Ruby libraries

### 2. Configuration

Create an initializer at `config/initializers/opentelemetry.rb`:

```ruby
# Configure OpenTelemetry with sensible defaults for Rails
require 'opentelemetry/sdk'
require 'opentelemetry/exporter/otlp'
require 'opentelemetry/instrumentation/all'

OpenTelemetry::SDK.configure do |c|
  # Service name identifies this application in traces
  c.service_name = ENV.fetch('OTEL_SERVICE_NAME', 'med-tracker')
  
  # Service version for tracking deployments
  c.service_version = ENV.fetch('OTEL_SERVICE_VERSION', '1.0.0')
  
  # Configure OTLP exporter (can export to Jaeger, Honeycomb, etc.)
  c.use 'OpenTelemetry::Exporter::OTLP'
  
  # Auto-instrument all supported libraries
  c.use_all
end
```

### 3. Environment Configuration

Environment variables for configuration:

```bash
# Required
OTEL_SERVICE_NAME=med-tracker
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318

# Optional
OTEL_SERVICE_VERSION=1.0.0
OTEL_TRACES_EXPORTER=otlp
OTEL_METRICS_EXPORTER=otlp
OTEL_LOGS_EXPORTER=otlp
OTEL_RESOURCE_ATTRIBUTES=deployment.environment=development
```

### 4. Development Setup with Jaeger

For local development, use Jaeger with Docker:

```bash
docker run -d --name jaeger \
  -e COLLECTOR_OTLP_ENABLED=true \
  -p 16686:16686 \
  -p 4318:4318 \
  jaegertracing/all-in-one:latest
```

Access Jaeger UI at: http://localhost:16686

## Custom Instrumentation

For custom business logic tracing:

```ruby
require 'opentelemetry/api'

class MedicationTakesController < ApplicationController
  def create
    tracer = OpenTelemetry.tracer_provider.tracer('med-tracker')
    
    tracer.in_span('create_medication_take') do |span|
      span.set_attribute('medication.id', params[:medication_id])
      span.set_attribute('person.id', current_user.person.id)
      
      # Your business logic here
      @medication_take = MedicationTake.create!(medication_take_params)
      
      span.add_event('medication_take_created', attributes: {
        'take.id' => @medication_take.id
      })
    end
  end
end
```

## Testing Considerations

### Test Environment Configuration

In `config/environments/test.rb`, you may want to disable OTel or use a no-op exporter:

```ruby
# Disable OpenTelemetry in tests
ENV['OTEL_TRACES_EXPORTER'] = 'none'
ENV['OTEL_METRICS_EXPORTER'] = 'none'
```

Or configure in the initializer:
```ruby
unless Rails.env.test?
  OpenTelemetry::SDK.configure do |c|
    # ... configuration
  end
end
```

## Production Considerations

### 1. Sampling
For high-traffic applications, configure sampling to reduce overhead:

```ruby
c.add_span_processor(
  OpenTelemetry::SDK::Trace::Export::BatchSpanProcessor.new(
    exporter,
    max_queue_size: 2048,
    schedule_delay_millis: 5000
  )
)

# Configure sampling rate (e.g., 10% of traces)
c.sampler = OpenTelemetry::SDK::Trace::Samplers::ParentBased.new(
  root: OpenTelemetry::SDK::Trace::Samplers::TraceIdRatioBased.new(0.1)
)
```

### 2. Resource Attributes
Add resource attributes for better filtering:

```ruby
c.resource = OpenTelemetry::SDK::Resources::Resource.create({
  'service.name' => 'med-tracker',
  'service.version' => Rails.application.config.version,
  'deployment.environment' => Rails.env,
  'host.name' => Socket.gethostname
})
```

### 3. Backend Options

Popular backend options:
- **Jaeger**: Open-source, self-hosted, great for development
- **Honeycomb**: Commercial, excellent UX, generous free tier
- **New Relic**: Commercial, full APM suite
- **Datadog**: Commercial, full observability platform
- **Grafana Tempo**: Open-source, works with Grafana stack
- **AWS X-Ray**: AWS-native solution
- **Google Cloud Trace**: GCP-native solution

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

3. **Data Retention**: Configure appropriate retention policies on backend

## Integration with Existing Stack

### Compatible with Current Stack
- ✅ Rails 8.x - Full support
- ✅ Puma - Works seamlessly
- ✅ SQLite3 - Supported (via ActiveRecord instrumentation)
- ✅ Turbo/Stimulus - No conflicts
- ✅ RSpec - Can be disabled in tests or use test exporter

### Additional Considerations
- Phlex views: Traced via ActionView instrumentation
- Solid Queue: Background job tracing available via `opentelemetry-instrumentation-active_job`
- Solid Cache: Cache operations traced via ActiveSupport instrumentation

## Recommended Implementation Steps

1. ✅ Add OpenTelemetry gems to Gemfile
2. ✅ Create initializer with basic configuration
3. ✅ Set up local Jaeger instance for development
4. ✅ Configure environment variables
5. ✅ Test automatic instrumentation
6. ✅ Add custom spans for critical business logic
7. ✅ Configure for test environment
8. ✅ Document usage for team
9. ✅ Choose and configure production backend
10. ✅ Set up monitoring dashboards

## Alternative Approaches Considered

### 1. APM-Specific Agents
- **New Relic Ruby Agent**: Vendor lock-in, but excellent features
- **Datadog APM**: Similar to New Relic
- **Scout APM**: Rails-focused, simpler setup

**Decision**: OpenTelemetry chosen for vendor neutrality and standardization

### 2. Custom Logging
- Using Rails logger with structured logging
- **Drawback**: No distributed tracing, harder to correlate events

### 3. Rack-Attack + Custom Metrics
- Simple middleware-based approach
- **Drawback**: Limited visibility, no automatic instrumentation

## References and Resources

### Official Documentation
- [OpenTelemetry Ruby Documentation](https://opentelemetry.io/docs/instrumentation/ruby/)
- [OpenTelemetry Ruby GitHub](https://github.com/open-telemetry/opentelemetry-ruby)
- [OpenTelemetry Specification](https://opentelemetry.io/docs/specs/otel/)

### Rails-Specific Guides
- [OpenTelemetry Rails Instrumentation](https://github.com/open-telemetry/opentelemetry-ruby-contrib/tree/main/instrumentation/rails)
- [Ruby Automatic Instrumentation](https://opentelemetry.io/docs/instrumentation/ruby/automatic/)

### Backend Setup Guides
- [Jaeger Getting Started](https://www.jaegertracing.io/docs/latest/getting-started/)
- [Honeycomb OpenTelemetry Guide](https://docs.honeycomb.io/getting-data-in/opentelemetry/)
- [Grafana Tempo Setup](https://grafana.com/docs/tempo/latest/getting-started/)

### Community Resources
- [OpenTelemetry CNCF Slack](https://cloud-native.slack.com)
- [Ruby OTel Examples](https://github.com/open-telemetry/opentelemetry-ruby-contrib/tree/main/examples)

## Conclusion

OpenTelemetry provides a robust, vendor-neutral solution for adding observability to the MedTracker Rails application. The implementation is straightforward with automatic instrumentation for most of the stack, and the framework allows for custom instrumentation where needed. The minimal performance overhead and flexible configuration make it suitable for both development and production environments.

The recommended approach is to start with automatic instrumentation and add custom spans for critical business logic as needed. This provides immediate value with minimal code changes while allowing for more detailed instrumentation over time.
