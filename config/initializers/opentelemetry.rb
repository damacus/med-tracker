# frozen_string_literal: true

# OpenTelemetry configuration for MedTracker
# Provides distributed tracing and observability
#
# Environment variables:
#   OTEL_EXPORTER_OTLP_ENDPOINT - OTLP collector endpoint (default: http://localhost:4318)
#   OTEL_SERVICE_NAME - Service name for traces (default: medtracker)
#   OTEL_ENABLED - Enable/disable OpenTelemetry (default: true in production, false otherwise)
#   OTEL_LOG_LEVEL - Log level for OpenTelemetry (default: info)

return unless ENV.fetch('OTEL_ENABLED', Rails.env.production? ? 'true' : 'false') == 'true'

require 'opentelemetry/sdk'
require 'opentelemetry/exporter/otlp'
require 'opentelemetry/instrumentation/all'

OpenTelemetry::SDK.configure do |c|
  c.service_name = ENV.fetch('OTEL_SERVICE_NAME', 'medtracker')

  # Configure the OTLP exporter
  c.add_span_processor(
    OpenTelemetry::SDK::Trace::Export::BatchSpanProcessor.new(
      OpenTelemetry::Exporter::OTLP::Exporter.new(
        endpoint: ENV.fetch('OTEL_EXPORTER_OTLP_ENDPOINT', 'http://localhost:4318/v1/traces')
      )
    )
  )

  # Auto-instrument all supported libraries
  c.use_all({
    'OpenTelemetry::Instrumentation::Rails' => {
      enable_recognized_route_pattern_attribute: true
    },
    'OpenTelemetry::Instrumentation::ActiveRecord' => {
      db_statement: :obfuscate
    },
    'OpenTelemetry::Instrumentation::ActionPack' => {},
    'OpenTelemetry::Instrumentation::ActionView' => {},
    'OpenTelemetry::Instrumentation::ActiveJob' => {},
    'OpenTelemetry::Instrumentation::ActiveSupport' => {},
    'OpenTelemetry::Instrumentation::Net::HTTP' => {},
    'OpenTelemetry::Instrumentation::PG' => {
      db_statement: :obfuscate
    },
    'OpenTelemetry::Instrumentation::Rack' => {},
    'OpenTelemetry::Instrumentation::Concurrent::Ruby' => {}
  })
end

Rails.logger.info 'OpenTelemetry initialized successfully'
