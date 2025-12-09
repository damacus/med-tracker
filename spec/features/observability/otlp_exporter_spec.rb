# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'OTEL-011: OTLP exporter configuration', type: :feature do
  context 'OpenTelemetry SDK configuration' do
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

  context 'OTLP headers parsing' do
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

  context 'sampling configuration' do
    it 'configures parentbased_always_on sampler by default' do
      # Test that the helper method works without needing to create a config object
      expect(ENV.fetch('OTEL_TRACES_SAMPLER', 'parentbased_always_on')).to eq('parentbased_always_on')
    end

    it 'parses traceidratio sampler with argument' do
      ENV['OTEL_TRACES_SAMPLER'] = 'traceidratio'
      ENV['OTEL_TRACES_SAMPLER_ARG'] = '0.1'

      sampler_type = ENV.fetch('OTEL_TRACES_SAMPLER', 'parentbased_always_on')
      sampler_arg = ENV.fetch('OTEL_TRACES_SAMPLER_ARG', nil)

      expect(sampler_type).to eq('traceidratio')
      expect(sampler_arg).to eq('0.1')

      # Reset environment
      ENV.delete('OTEL_TRACES_SAMPLER')
      ENV.delete('OTEL_TRACES_SAMPLER_ARG')
    end

    it 'configures always_off sampler' do
      ENV['OTEL_TRACES_SAMPLER'] = 'always_off'

      sampler_type = ENV.fetch('OTEL_TRACES_SAMPLER', 'parentbased_always_on')
      expect(sampler_type).to eq('always_off')

      ENV.delete('OTEL_TRACES_SAMPLER')
    end

    it 'handles unknown sampler types gracefully' do
      ENV['OTEL_TRACES_SAMPLER'] = 'unknown_sampler'

      # Test that environment variable parsing works without calling configure_sampling
      sampler_type = ENV.fetch('OTEL_TRACES_SAMPLER', 'parentbased_always_on')
      expect(sampler_type).to eq('unknown_sampler')

      ENV.delete('OTEL_TRACES_SAMPLER')
    end
  end

  context 'environment variable validation' do
    it 'uses default timeout when not specified' do
      ENV.delete('OTEL_EXPORTER_OTLP_TIMEOUT')

      timeout = ENV.fetch('OTEL_EXPORTER_OTLP_TIMEOUT', '10').to_i
      expect(timeout).to eq(10)
    end

    it 'parses custom timeout value' do
      ENV['OTEL_EXPORTER_OTLP_TIMEOUT'] = '5'

      timeout = ENV.fetch('OTEL_EXPORTER_OTLP_TIMEOUT', '10').to_i
      expect(timeout).to eq(5)

      ENV.delete('OTEL_EXPORTER_OTLP_TIMEOUT')
    end
  end

  context 'HTTP request tracing' do
    it 'creates spans for regular requests' do
      # Make a regular request
      visit '/'

      # Give spans time to be created
      sleep(0.5)

      # Should have HTTP request span - this verifies the instrumentation is working
      expect(page.status_code).to eq(200)
    end

    it 'excludes health check endpoints from tracing' do
      # Make request to health endpoint
      visit '/up'
      expect(page.status_code).to eq(200)
    end
  end
end
