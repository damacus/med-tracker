# frozen_string_literal: true

require 'rails_helper'

RSpec.describe OpenTelemetry, type: :configuration do
  describe 'SDK initialization' do
    it 'configures OpenTelemetry with service name medtracker' do
      expect(described_class.tracer_provider).to be_a(OpenTelemetry::SDK::Trace::TracerProvider)

      resource = described_class.tracer_provider.resource
      service_name = resource.attribute_enumerator.find { |k, _| k == 'service.name' }&.last
      expect(service_name).to eq('medtracker')
    end

    it 'sets resource attributes correctly' do
      resource = described_class.tracer_provider.resource

      attributes = resource.attribute_enumerator.to_h
      expect(attributes).to include('service.name' => 'medtracker')
    end
  end

  describe 'instrumentation' do
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

    it 'enables Rack instrumentation' do
      instrumentation = OpenTelemetry::Instrumentation::Rack::Instrumentation.instance
      expect(instrumentation).not_to be_nil
      expect(instrumentation.installed?).to be(true)
    end
  end

  describe 'W3C trace context propagation' do
    it 'uses W3C TraceContext propagator' do
      propagators = described_class.propagation
      expect(propagators).to be_a(OpenTelemetry::Context::Propagation::CompositeTextMapPropagator)
    end
  end
end
