# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'OTEL-011: OTLP exporter configuration' do
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

    it 'enables Rack instrumentation with untraced endpoints' do
      instrumentation = OpenTelemetry::Instrumentation::Rack::Instrumentation.instance
      expect(instrumentation).not_to be_nil
      expect(instrumentation.installed?).to be(true)
    end

    it 'uses W3C TraceContext propagator' do
      propagators = OpenTelemetry.propagation
      expect(propagators).to be_a(OpenTelemetry::Context::Propagation::CompositeTextMapPropagator)
    end
  end

  context 'when parsing OTLP headers' do
    it 'parses headers string correctly' do
      headers_string = 'authorization=Bearer token123,x-custom-header=value'
      expected = {
        'authorization' => 'Bearer token123',
        'x-custom-header' => 'value'
      }

      result = OpenTelemetryConfig.parse_otlp_headers(headers_string)
      expect(result).to eq(expected)
    end

    it 'handles empty headers string' do
      result = OpenTelemetryConfig.parse_otlp_headers('')
      expect(result).to eq({})
    end

    it 'handles nil headers string' do
      result = OpenTelemetryConfig.parse_otlp_headers(nil)
      expect(result).to eq({})
    end

    it 'handles malformed headers gracefully' do
      headers_string = 'invalid-header,noequals'
      expected = {
        'invalid-header' => '',
        'noequals' => ''
      }

      result = OpenTelemetryConfig.parse_otlp_headers(headers_string)
      expect(result).to eq(expected)
    end
  end
end
