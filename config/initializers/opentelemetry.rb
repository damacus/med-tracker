# frozen_string_literal: true

require 'opentelemetry-api'
require 'otel/allowlisted_span_exporter'

OpenTelemetry.logger = Logger.new($stdout, level: Rails.env.test? ? Logger::WARN : Logger::ERROR)

module OpenTelemetryConfig
  module_function

  def parse_otlp_headers(headers_string)
    return {} unless headers_string.present?

    headers_string.split(',').each_with_object({}) do |header, hash|
      key, value = header.split('=', 2)
      if key && value
        hash[key.strip] = value.strip
      elsif key
        hash[key.strip] = ''
      end
    end
  end
end

if Rails.env.test?
  require 'opentelemetry/sdk'
  require 'opentelemetry/instrumentation/active_record'
  require 'opentelemetry/instrumentation/pg'
  require 'opentelemetry/instrumentation/rack'
  require 'opentelemetry/instrumentation/rails'

  OpenTelemetry::SDK.configure do |c|
    c.service_name = 'medtracker-test'
    c.service_version = 'test'

    c.use('OpenTelemetry::Instrumentation::Rails')
    c.use('OpenTelemetry::Instrumentation::ActiveRecord')
    c.use(
      'OpenTelemetry::Instrumentation::Rack',
      untraced_endpoints: ['/up', '/health', '/healthz', '/ready', '/live'],
      record_frontend_span: true
    )
    c.use(
      'OpenTelemetry::Instrumentation::PG',
      db_statement: :obfuscate,
      peer_service: 'postgresql'
    )

    c.resource = OpenTelemetry::SDK::Resources::Resource.create(
      'service.name' => 'medtracker-test',
      'service.namespace' => 'medtracker',
      'deployment.environment' => 'test'
    )
  end

elsif Rails.env.production? || ENV['OTEL_EXPORTER_OTLP_ENDPOINT'].present?
  require 'opentelemetry/sdk'
  require 'opentelemetry/exporter/otlp'
  require 'opentelemetry/instrumentation/active_record'
  require 'opentelemetry/instrumentation/net/http'
  require 'opentelemetry/instrumentation/pg'
  require 'opentelemetry/instrumentation/rack'
  require 'opentelemetry/instrumentation/rails'

  otlp_endpoint = ENV.fetch('OTEL_EXPORTER_OTLP_ENDPOINT', nil)
  otlp_headers = ENV.fetch('OTEL_EXPORTER_OTLP_HEADERS', nil)
  otlp_timeout = ENV.fetch('OTEL_EXPORTER_OTLP_TIMEOUT', '10').to_i

  Rails.logger.info "[OpenTelemetry] Configuring OTLP exporter: #{otlp_endpoint}" if otlp_endpoint.present?

  OpenTelemetry::SDK.configure do |c|
    c.service_name = 'medtracker'
    c.service_version = ENV.fetch('APP_VERSION', '1.0.0')

    if otlp_endpoint.present?
      span_exporter = OpenTelemetry::Exporter::OTLP::Exporter.new(
        endpoint: otlp_endpoint,
        headers: OpenTelemetryConfig.parse_otlp_headers(otlp_headers),
        timeout: otlp_timeout
      )
      span_exporter = Otel::AllowlistedSpanExporter.new(span_exporter)

      c.add_span_processor(
        OpenTelemetry::SDK::Trace::Export::BatchSpanProcessor.new(
          exporter: span_exporter,
          max_queue_size: 2048,
          scheduled_delay_millis: 5000,
          max_export_batch_size: 512
        )
      )
    end

    c.use('OpenTelemetry::Instrumentation::Rails')
    c.use('OpenTelemetry::Instrumentation::ActiveRecord')
    c.use(
      'OpenTelemetry::Instrumentation::Rack',
      untraced_endpoints: ['/up', '/health', '/healthz', '/ready', '/live'],
      record_frontend_span: true
    )
    c.use(
      'OpenTelemetry::Instrumentation::PG',
      db_statement: :obfuscate,
      peer_service: 'postgresql'
    )
    c.use(
      'OpenTelemetry::Instrumentation::Net::HTTP',
      untraced_hosts: ['127.0.0.1', 'localhost']
    )

    c.resource = OpenTelemetry::SDK::Resources::Resource.create(
      'service.name' => 'medtracker',
      'service.namespace' => 'medtracker',
      'deployment.environment' => Rails.env.to_s,
      'host.name' => Socket.gethostname,
      'process.pid' => Process.pid.to_s
    )
  end
end
