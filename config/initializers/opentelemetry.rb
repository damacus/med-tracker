# frozen_string_literal: true

# Skip OpenTelemetry in development/test - no collector running
# Only enable in production where OTLP endpoint is configured
unless Rails.env.production? || ENV['OTEL_EXPORTER_OTLP_ENDPOINT'].present?
  return
end

# Load OpenTelemetry API first and set logger to suppress deprecation warnings
# The opentelemetry-helpers-sql-obfuscation gem emits a deprecation warning on load
# This is harmless - the gem was renamed to opentelemetry-helpers-sql-processor
require 'opentelemetry-api'
OpenTelemetry.logger = Logger.new($stdout, level: Logger::ERROR)

require 'opentelemetry/sdk'
require 'opentelemetry/exporter/otlp'
require 'opentelemetry/instrumentation/all'

# Configure OpenTelemetry SDK for MedTracker observability
OpenTelemetry::SDK.configure do |c|
  c.service_name = 'medtracker'
  c.service_version = ENV.fetch('APP_VERSION', '1.0.0')

  # Enable all available instrumentations with sensible defaults
  c.use_all(
    # Configure Rails instrumentation (uses default options)
    'OpenTelemetry::Instrumentation::Rails' => {},
    # Configure ActiveRecord instrumentation (uses default options)
    'OpenTelemetry::Instrumentation::ActiveRecord' => {},
    # Configure Rack instrumentation with health check filtering
    'OpenTelemetry::Instrumentation::Rack' => {
      untraced_endpoints: ['/up', '/health', '/healthz', '/ready', '/live'],
      record_frontend_span: true
    },
    # Configure PG instrumentation
    'OpenTelemetry::Instrumentation::PG' => {
      db_statement: :include,
      peer_service: 'postgresql'
    },
    # Configure Net::HTTP instrumentation
    'OpenTelemetry::Instrumentation::Net::HTTP' => {
      untraced_hosts: ['127.0.0.1', 'localhost']
    }
  )

  # Configure resource attributes - ensure all values are strings
  c.resource = OpenTelemetry::SDK::Resources::Resource.create(
    'service.namespace' => 'medtracker',
    'deployment.environment' => Rails.env.to_s
  )
end
