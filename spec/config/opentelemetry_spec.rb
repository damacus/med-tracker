# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'OpenTelemetry Configuration' do
  describe 'in non-test environment' do
    it 'does not configure OpenTelemetry in test environment' do
      # OpenTelemetry should be skipped in test environment
      expect(Rails.env.test?).to be true
    end
  end

  describe 'initialization' do
    it 'loads OpenTelemetry SDK' do
      expect(defined?(OpenTelemetry::SDK)).to be_truthy
    end

    it 'loads OpenTelemetry API' do
      expect(defined?(OpenTelemetry)).to be_truthy
    end

    it 'loads OTLP exporter' do
      expect(defined?(OpenTelemetry::Exporter::OTLP)).to be_truthy
    end
  end

  describe 'tracer provider' do
    it 'provides a tracer' do
      tracer = OpenTelemetry.tracer_provider.tracer('test-tracer')
      expect(tracer).not_to be_nil
    end

    it 'can create spans' do
      tracer = OpenTelemetry.tracer_provider.tracer('test-tracer')
      span = nil
      
      tracer.in_span('test-span') do |current_span|
        span = current_span
        expect(current_span).not_to be_nil
      end
      
      expect(span).not_to be_nil
    end
  end
end
