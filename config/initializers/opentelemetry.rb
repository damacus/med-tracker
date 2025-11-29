# frozen_string_literal: true

# Configure OpenTelemetry for observability and distributed tracing
# See docs/opentelemetry_research.md for detailed information

# Skip configuration in test environment to avoid overhead
return if Rails.env.test?

require 'opentelemetry/sdk'
require 'opentelemetry/exporter/otlp'
require 'opentelemetry/instrumentation/all'

OpenTelemetry::SDK.configure do |c|
  # Service name identifies this application in traces
  c.service_name = ENV.fetch('OTEL_SERVICE_NAME', 'med-tracker')

  # Service version for tracking deployments
  c.service_version = ENV.fetch('OTEL_SERVICE_VERSION', '1.0.0')

  # Configure resource attributes for better filtering and grouping
  c.resource = OpenTelemetry::SDK::Resources::Resource.create(
    'deployment.environment' => Rails.env.to_s,
    'service.namespace' => ENV.fetch('OTEL_SERVICE_NAMESPACE', 'default')
  )

  # Auto-instrument all supported libraries including:
  # - Rails (ActionController, ActionView, ActiveRecord, etc.)
  # - Rack
  # - Net::HTTP
  # - OTLP Exporter
  # - And many more
  c.use_all
end
