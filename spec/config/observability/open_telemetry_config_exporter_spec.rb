# frozen_string_literal: true

require 'rails_helper'

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
      initializer = Rails.root.join('config/initializers/opentelemetry.rb').read
      production_configuration = initializer.split('elsif Rails.env.production?').last
      resource_assignment = production_configuration.index(
        'c.resource = OpenTelemetry::SDK::Resources::Resource.create'
      )

      materializing_configuration_calls = [
        'OpenTelemetryConfig.apply_trace_sampler',
        'c.add_span_processor'
      ].filter_map { |call| production_configuration.index(call) }

      expect(materializing_configuration_calls).not_to be_empty
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
end
