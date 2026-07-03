# frozen_string_literal: true

require 'opentelemetry-api'
require 'otel/allowlisted_span_exporter'
require 'otel/critical_path_sampler'
require 'otel/exception_recorder'
require 'otel/log_correlation'

OpenTelemetry.logger = Logger.new($stdout, level: Rails.env.test? ? Logger::WARN : Logger::ERROR)

module OpenTelemetryConfig
  DEFAULT_PRODUCTION_TRACE_SAMPLE_RATE = 0.1
  DEFAULT_TRACE_SAMPLE_RATE = 1.0
  DEFAULT_CRITICAL_TRACE_MATCHERS = [
    '/login',
    '/logout',
    '/create-account',
    '/api/v1/auth/login',
    '/api/v1/auth/refresh',
    '/api/v1/auth/logout',
    '/medication_takes',
    '/take_medication',
    '/offline/medication_takes',
    '/admin/audit_logs',
    '/platform/support_access_sessions',
    'medication_take.'
  ].freeze

  module_function

  def parse_otlp_headers(headers_string)
    return {} if headers_string.blank?

    headers_string.split(',').each_with_object({}) do |header, hash|
      key, value = header.split('=', 2)
      if key && value
        hash[key.strip] = value.strip
      elsif key
        hash[key.strip] = ''
      end
    end
  end

  def trace_sampler(environment: Rails.env, env: ENV)
    Otel::CriticalPathSampler.new(
      delegate: delegate_trace_sampler(environment:, env:),
      critical_path_matchers: critical_trace_matchers(env:)
    )
  end

  def apply_trace_sampler(configurator, sampler)
    configurator.send(:tracer_provider).sampler = sampler
  end

  def trace_sample_rate(environment: Rails.env, env: ENV)
    configured_rate = env['MEDTRACKER_OTEL_TRACE_SAMPLE_RATE'].presence || env['OTEL_TRACES_SAMPLER_ARG'].presence

    parse_trace_sample_rate(configured_rate, fallback: default_trace_sample_rate(environment))
  end

  def critical_trace_matchers(env: ENV)
    configured_matchers = env['MEDTRACKER_OTEL_CRITICAL_TRACE_MATCHERS']
    return DEFAULT_CRITICAL_TRACE_MATCHERS if configured_matchers.blank?
    return [] if configured_matchers.strip.casecmp?('none')

    configured_matchers.split(',').map(&:strip).compact_blank
  end

  def delegate_trace_sampler(environment: Rails.env, env: ENV)
    sample_rate = trace_sample_rate(environment:, env:)

    named_trace_sampler(env['OTEL_TRACES_SAMPLER'].presence, sample_rate)
  end

  def named_trace_sampler(sampler_name, sample_rate)
    case sampler_name
    when 'always_on'
      OpenTelemetry::SDK::Trace::Samplers::ALWAYS_ON
    when 'always_off'
      OpenTelemetry::SDK::Trace::Samplers::ALWAYS_OFF
    when 'traceidratio'
      OpenTelemetry::SDK::Trace::Samplers.trace_id_ratio_based(sample_rate)
    when 'parentbased_always_on'
      parent_based_sampler(OpenTelemetry::SDK::Trace::Samplers::ALWAYS_ON)
    when 'parentbased_always_off'
      parent_based_sampler(OpenTelemetry::SDK::Trace::Samplers::ALWAYS_OFF)
    else
      parent_based_sampler(OpenTelemetry::SDK::Trace::Samplers.trace_id_ratio_based(sample_rate))
    end
  end

  def parent_based_sampler(root_sampler)
    OpenTelemetry::SDK::Trace::Samplers.parent_based(root: root_sampler)
  end

  def default_trace_sample_rate(environment)
    environment.to_s == 'production' ? DEFAULT_PRODUCTION_TRACE_SAMPLE_RATE : DEFAULT_TRACE_SAMPLE_RATE
  end

  def parse_trace_sample_rate(value, fallback:)
    return fallback if value.blank?

    Float(value).clamp(0.0, 1.0)
  rescue ArgumentError, TypeError
    fallback
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
    OpenTelemetryConfig.apply_trace_sampler(c, OpenTelemetryConfig.trace_sampler)

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
