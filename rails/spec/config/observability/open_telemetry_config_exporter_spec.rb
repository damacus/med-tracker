# frozen_string_literal: true

require 'rails_helper'
require 'openssl'

RSpec.describe OpenTelemetryConfig do
  context 'when configuring the OpenTelemetry SDK' do
    it 'configures OpenTelemetry with service name medtracker-test' do
      expect(OpenTelemetry.tracer_provider).to be_a(OpenTelemetry::SDK::Trace::TracerProvider)

      resource = OpenTelemetry.tracer_provider.resource
      service_name = resource.attribute_enumerator.find { |k, _| k == 'service.name' }&.last
      expect(service_name).to eq('medtracker-test')
    end

    it 'sets resource attributes correctly' do
      resource = OpenTelemetry.tracer_provider.resource
      attributes = resource.attribute_enumerator.to_h

      expect(attributes).to include(
        'service.name' => 'medtracker-test',
        'service.namespace' => 'medtracker',
        'deployment.environment' => 'test'
      )
    end

    it 'assigns the production resource before configuring the sampler or exporter' do
      resource_assignment = production_configuration_position(
        'c.resource = OpenTelemetry::SDK::Resources::Resource.create'
      )
      sampler_assignment = production_configuration_position(
        'OpenTelemetryConfig.apply_trace_sampler(c, OpenTelemetryConfig.trace_sampler)'
      )

      materializing_configuration_calls = [
        sampler_assignment,
        production_configuration_position('c.add_span_processor')
      ].compact

      expect(production_configuration).not_to include('OpenTelemetry.tracer_provider.sampler =')
      expect(sampler_assignment).to be_present
      materializing_configuration_calls.each do |configuration_call|
        expect(resource_assignment).to be < configuration_call
      end
    end

    it 'enables Rails instrumentation' do
      instrumentation = OpenTelemetry::Instrumentation::Rails::Instrumentation.instance
      expect(instrumentation).not_to be_nil
      expect(instrumentation.installed?).to be(true)
    end

    it 'enables ActiveRecord instrumentation' do
      instrumentation = OpenTelemetry::Instrumentation::ActiveRecord::Instrumentation.instance
      expect(instrumentation).not_to be_nil
      expect(instrumentation.installed?).to be(true)
    end

    it 'enables Active Job instrumentation with linked trace propagation' do
      instrumentation = OpenTelemetry::Instrumentation::ActiveJob::Instrumentation.instance

      expect(instrumentation).to be_installed
      expect(instrumentation.instance_variable_get(:@config)).to include(
        propagation_style: :link,
        span_naming: :job_class
      )
    end

    it 'enables Rack instrumentation with untraced endpoints' do
      instrumentation = OpenTelemetry::Instrumentation::Rack::Instrumentation.instance
      expect(instrumentation).not_to be_nil
      expect(instrumentation.installed?).to be(true)
    end

    it 'uses W3C TraceContext propagator' do
      propagators = OpenTelemetry.propagation
      expect(propagators).to be_a(OpenTelemetry::Context::Propagation::CompositeTextMapPropagator)
    end

    it 'does not install span processors that export traces in test' do
      span_processors = OpenTelemetry.tracer_provider.instance_variable_get(:@span_processors)

      expect(span_processors).to be_empty
    end

    it 'installs database pool timeout instrumentation' do
      expect(ActiveRecord::ConnectionAdapters::ConnectionPool).to be < Otel::ConnectionPoolTimeoutInstrumentation
      expect(Otel::DatabaseConnectionPoolMetrics.current).to be_present
      expect(Otel::DatabaseConnectionPoolMetrics.current.send(:connection_pools)).to include(ActiveRecord::Base.connection_pool)
    end

    it 'resolves connection pools across every Active Record role' do
      handler = instance_spy(ActiveRecord::ConnectionAdapters::ConnectionHandler, connection_pool_list: [])
      allow(ActiveRecord::Base).to receive(:connection_handler).and_return(handler)

      Otel::DatabaseConnectionPoolMetrics.current.send(:connection_pools)

      expect(handler).to have_received(:connection_pool_list).with(:all)
    end
  end

  context 'when parsing OTLP headers' do
    it 'parses headers string correctly' do
      headers_string = 'authorization=Bearer token123,x-custom-header=value'
      expected = {
        'authorization' => 'Bearer token123',
        'x-custom-header' => 'value'
      }

      result = described_class.parse_otlp_headers(headers_string)
      expect(result).to eq(expected)
    end

    it 'handles empty headers string' do
      result = described_class.parse_otlp_headers('')
      expect(result).to eq({})
    end

    it 'handles nil headers string' do
      result = described_class.parse_otlp_headers(nil)
      expect(result).to eq({})
    end

    it 'handles malformed headers gracefully' do
      headers_string = 'invalid-header,noequals'
      expected = {
        'invalid-header' => '',
        'noequals' => ''
      }

      result = described_class.parse_otlp_headers(headers_string)
      expect(result).to eq(expected)
    end
  end

  context 'when handling exporter errors' do
    let(:error_output) { StringIO.new }

    around do |example|
      original_logger = OpenTelemetry.logger
      OpenTelemetry.logger = Observability::DatasetLogger.new(
        error_output,
        dataset: 'medtracker.opentelemetry',
        level: Logger::ERROR
      )

      example.run
    ensure
      OpenTelemetry.logger = original_logger
    end

    it 'preserves only the fixed type of an allowlisted exporter exception' do
      error = OpenSSL::SSL::SSLError.new('private exporter endpoint')

      OpenTelemetry.handle_error(exception: error, message: 'private exporter payload')

      record = JSON.parse(error_output.string)
      expect(record).to include(
        'event.reason' => 'export_failed',
        'error.type' => 'OpenSSL::SSL::SSLError'
      )
      expect(record.to_json).not_to include('private', 'endpoint', 'payload')
    end
  end

  def production_configuration
    Rails.root.join('config/initializers/opentelemetry.rb').read.split('elsif Rails.env.production?').last
  end

  def production_configuration_position(source)
    production_configuration.index(source)
  end
end
