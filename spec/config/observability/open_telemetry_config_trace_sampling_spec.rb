# frozen_string_literal: true

require 'rails_helper'

RSpec.describe OpenTelemetryConfig do
  describe '.trace_sample_rate' do
    it 'uses the production baseline by default' do
      rate = described_class.trace_sample_rate(environment: 'production', env: {})

      expect(rate).to eq(0.1)
    end

    it 'uses the app-specific override when present' do
      env = { 'MEDTRACKER_OTEL_TRACE_SAMPLE_RATE' => '0.25' }

      rate = described_class.trace_sample_rate(environment: 'production', env: env)

      expect(rate).to eq(0.25)
    end

    it 'falls back to the environment default for invalid overrides' do
      env = { 'MEDTRACKER_OTEL_TRACE_SAMPLE_RATE' => 'invalid' }

      rate = described_class.trace_sample_rate(environment: 'production', env: env)

      expect(rate).to eq(0.1)
    end
  end

  describe '.critical_trace_matchers' do
    it 'uses safe defaults when no override is configured' do
      matchers = described_class.critical_trace_matchers(env: {})

      expect(matchers).to include('/medication_takes', '/api/v1/auth/login', 'medication_take.')
    end

    it 'parses operator supplied matchers' do
      env = { 'MEDTRACKER_OTEL_CRITICAL_TRACE_MATCHERS' => '/login, /admin/audit_logs' }

      matchers = described_class.critical_trace_matchers(env: env)

      expect(matchers).to eq(['/login', '/admin/audit_logs'])
    end

    it 'allows operators to disable critical matcher overrides' do
      env = { 'MEDTRACKER_OTEL_CRITICAL_TRACE_MATCHERS' => 'none' }

      matchers = described_class.critical_trace_matchers(env: env)

      expect(matchers).to eq([])
    end
  end

  describe '.trace_sampler' do
    it 'builds a critical path sampler around the baseline sampler' do
      env = { 'MEDTRACKER_OTEL_TRACE_SAMPLE_RATE' => '0.2' }

      sampler = described_class.trace_sampler(environment: 'production', env: env)

      expect(sampler).to be_a(Otel::CriticalPathSampler)
      expect(sampler.description).to include('TraceIdRatioBased{0.200000}')
    end

    it 'honours the standard always_off sampler while retaining critical paths' do
      env = { 'OTEL_TRACES_SAMPLER' => 'always_off' }

      sampler = described_class.trace_sampler(environment: 'production', env: env)
      result = sampler.should_sample?(
        trace_id: '0af7651916cd43dd8448eb211c80319c',
        parent_context: OpenTelemetry::Context.empty,
        links: [],
        name: 'GET /households/:household_slug/dashboard',
        kind: :server,
        attributes: { 'http.route' => '/households/:household_slug/schedules/:schedule_id/medication_takes' }
      )

      expect(result).to be_sampled
    end
  end

  describe '.apply_trace_sampler' do
    it 'sets the sampler on the SDK tracer provider' do
      provider = Struct.new(:sampler).new
      configurator_class = Class.new do
        def initialize(provider)
          @provider = provider
        end

        private

        def tracer_provider
          @provider
        end
      end
      sampler = OpenTelemetry::SDK::Trace::Samplers::ALWAYS_ON

      described_class.apply_trace_sampler(configurator_class.new(provider), sampler)

      expect(provider.sampler).to eq(sampler)
    end
  end
end
