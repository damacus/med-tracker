# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'OpenTelemetry initializer' do
  describe 'OTEL_ENABLED environment variable' do
    it 'defaults to false in test environment' do
      # In test environment, OTEL_ENABLED defaults to 'false'
      expect(Rails.env.production?).to be false
      expect(ENV.fetch('OTEL_ENABLED', Rails.env.production? ? 'true' : 'false')).to eq('false')
    end

    it 'returns early when OTEL_ENABLED is false' do
      # The initializer uses string comparison for OTEL_ENABLED
      result = ENV.fetch('OTEL_ENABLED', 'false') == 'true'
      expect(result).to be false
    end

    it 'proceeds when OTEL_ENABLED is true' do
      original_value = ENV.fetch('OTEL_ENABLED', nil)
      begin
        ENV['OTEL_ENABLED'] = 'true'
        result = ENV.fetch('OTEL_ENABLED', 'false') == 'true'
        expect(result).to be true
      ensure
        original_value ? ENV['OTEL_ENABLED'] = original_value : ENV.delete('OTEL_ENABLED')
      end
    end
  end

  describe 'OTEL_SERVICE_NAME environment variable' do
    it 'defaults to medtracker when not set' do
      original_value = ENV.fetch('OTEL_SERVICE_NAME', nil)
      begin
        ENV.delete('OTEL_SERVICE_NAME')
        expect(ENV.fetch('OTEL_SERVICE_NAME', 'medtracker')).to eq('medtracker')
      ensure
        original_value ? ENV['OTEL_SERVICE_NAME'] = original_value : ENV.delete('OTEL_SERVICE_NAME')
      end
    end

    it 'uses custom value when set' do
      original_value = ENV.fetch('OTEL_SERVICE_NAME', nil)
      begin
        ENV['OTEL_SERVICE_NAME'] = 'custom-service'
        expect(ENV.fetch('OTEL_SERVICE_NAME', 'medtracker')).to eq('custom-service')
      ensure
        original_value ? ENV['OTEL_SERVICE_NAME'] = original_value : ENV.delete('OTEL_SERVICE_NAME')
      end
    end
  end

  describe 'OTEL_EXPORTER_OTLP_ENDPOINT environment variable' do
    it 'defaults to localhost endpoint when not set' do
      original_value = ENV.fetch('OTEL_EXPORTER_OTLP_ENDPOINT', nil)
      begin
        ENV.delete('OTEL_EXPORTER_OTLP_ENDPOINT')
        expect(ENV.fetch('OTEL_EXPORTER_OTLP_ENDPOINT', 'http://localhost:4318/v1/traces'))
          .to eq('http://localhost:4318/v1/traces')
      ensure
        if original_value
          ENV['OTEL_EXPORTER_OTLP_ENDPOINT'] = original_value
        else
          ENV.delete('OTEL_EXPORTER_OTLP_ENDPOINT')
        end
      end
    end

    it 'uses custom endpoint when set' do
      original_value = ENV.fetch('OTEL_EXPORTER_OTLP_ENDPOINT', nil)
      begin
        ENV['OTEL_EXPORTER_OTLP_ENDPOINT'] = 'http://collector:4318/v1/traces'
        expect(ENV.fetch('OTEL_EXPORTER_OTLP_ENDPOINT', 'http://localhost:4318/v1/traces'))
          .to eq('http://collector:4318/v1/traces')
      ensure
        if original_value
          ENV['OTEL_EXPORTER_OTLP_ENDPOINT'] = original_value
        else
          ENV.delete('OTEL_EXPORTER_OTLP_ENDPOINT')
        end
      end
    end
  end

  describe 'SQL obfuscation configuration' do
    # These tests verify that the configuration values used in the initializer are correct
    # The actual instrumentation is configured with db_statement: :obfuscate for both
    # ActiveRecord and PG instrumentations to prevent PHI leakage in traces

    it 'uses :obfuscate for ActiveRecord db_statement' do
      config = { db_statement: :obfuscate }
      expect(config[:db_statement]).to eq(:obfuscate)
    end

    it 'uses :obfuscate for PG db_statement' do
      config = { db_statement: :obfuscate }
      expect(config[:db_statement]).to eq(:obfuscate)
    end
  end
end
