# frozen_string_literal: true

unless Rails.env.production? || ENV['OTEL_EXPORTER_OTLP_ENDPOINT'].present?
  return
end

require 'opentelemetry-api'
OpenTelemetry.logger = Logger.new($stdout, level: Logger::ERROR)

require 'opentelemetry/sdk'
require 'opentelemetry/exporter/otlp'
require 'opentelemetry/instrumentation/all'

OpenTelemetry::SDK.configure do |c|
  c.service_name = 'medtracker'
  c.service_version = ENV.fetch('APP_VERSION', '1.0.0')

  c.use_all(
    'OpenTelemetry::Instrumentation::Rails' => {},
    'OpenTelemetry::Instrumentation::ActiveRecord' => {},
    'OpenTelemetry::Instrumentation::Rack' => {
      untraced_endpoints: ['/up', '/health', '/healthz', '/ready', '/live'],
      record_frontend_span: true
    },
    'OpenTelemetry::Instrumentation::PG' => {
      db_statement: :include,
      peer_service: 'postgresql'
    },
    'OpenTelemetry::Instrumentation::Net::HTTP' => {
      untraced_hosts: ['127.0.0.1', 'localhost']
    }
  )

  c.resource = OpenTelemetry::SDK::Resources::Resource.create(
    'service.namespace' => 'medtracker',
    'deployment.environment' => Rails.env.to_s
  )
end
