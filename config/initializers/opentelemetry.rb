# frozen_string_literal: true

require 'opentelemetry-api'

# Configure logger for OpenTelemetry
OpenTelemetry.logger = Logger.new($stdout, level: Rails.env.test? ? Logger::WARN : Logger::ERROR)

# Test environment uses in-memory exporters for verification
if Rails.env.test?
  require 'opentelemetry/sdk'
  require 'opentelemetry/instrumentation/all'

  OpenTelemetry::SDK.configure do |c|
    c.service_name = 'medtracker-test'
    c.service_version = 'test'

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
      }
    )

    c.resource = OpenTelemetry::SDK::Resources::Resource.create(
      'service.name' => 'medtracker-test',
      'service.namespace' => 'medtracker',
      'deployment.environment' => 'test'
    )
  end

# Production and non-production with OTLP endpoint configured
elsif Rails.env.production? || ENV['OTEL_EXPORTER_OTLP_ENDPOINT'].present?
  require 'opentelemetry/sdk'
  require 'opentelemetry/exporter/otlp'
  require 'opentelemetry/instrumentation/all'

  # Validate OTLP configuration
  otlp_endpoint = ENV.fetch('OTEL_EXPORTER_OTLP_ENDPOINT', nil)
  otlp_headers = ENV.fetch('OTEL_EXPORTER_OTLP_HEADERS', nil)
  otlp_timeout = ENV.fetch('OTEL_EXPORTER_OTLP_TIMEOUT', '10').to_i

  if otlp_endpoint.present?
    Rails.logger.info "[OpenTelemetry] Configuring OTLP exporter: #{otlp_endpoint}"
  end

  OpenTelemetry::SDK.configure do |c|
    c.service_name = 'medtracker'
    c.service_version = ENV.fetch('APP_VERSION', '1.0.0')

    # Configure OTLP exporter for traces
    if otlp_endpoint.present?
      span_exporter = OpenTelemetry::Exporter::OTLP::Exporter.new(
        endpoint: otlp_endpoint,
        headers: parse_otlp_headers(otlp_headers),
        timeout: otlp_timeout
      )

      c.add_span_processor(
        OpenTelemetry::SDK::Trace::Export::BatchSpanProcessor.new(
          exporter: span_exporter,
          max_queue_size: 2048,
          scheduled_delay_millis: 5000,
          max_export_batch_size: 512
        )
      )
    end

    # Configure sampling (commented out until correct sampler classes are identified)
    # configure_sampling(c)

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
      'service.name' => 'medtracker',
      'service.namespace' => 'medtracker',
      'deployment.environment' => Rails.env.to_s,
      'host.name' => Socket.gethostname,
      'process.pid' => Process.pid.to_s
    )
  end
end

module OpenTelemetryConfig
  module_function

  # Helper methods for OpenTelemetry configuration
  def parse_otlp_headers(headers_string)
    return {} unless headers_string.present?

    # Parse headers in format "key1=value1,key2=value2"
    headers_string.split(',').each_with_object({}) do |header, hash|
      key, value = header.split('=', 2)
      if key && value
        hash[key.strip] = value.strip
      elsif key
        # Handle case where there's no equals sign - treat as empty value
        hash[key.strip] = ''
      end
    end
  end
end
