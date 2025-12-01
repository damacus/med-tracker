# frozen_string_literal: true

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

# Log OpenTelemetry initialization
Rails.logger.info '[OpenTelemetry] SDK initialized with service name: medtracker' if defined?(Rails.logger) && Rails.logger
