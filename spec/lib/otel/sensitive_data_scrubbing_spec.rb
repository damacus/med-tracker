# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'OTEL-014: Sensitive data scrubbed from traces' do # rubocop:disable RSpec/DescribeClass
  describe 'PG instrumentation configuration' do
    it 'obfuscates SQL statements instead of including raw SQL' do
      pg_instrumentation = OpenTelemetry::Instrumentation::PG::Instrumentation.instance
      expect(pg_instrumentation).to be_installed

      config = pg_instrumentation.instance_variable_get(:@config)
      expect(config[:db_statement]).to eq(:obfuscate)
    end
  end

  describe 'Rack instrumentation configuration' do
    it 'does not record request headers that may contain sensitive data' do
      rack_instrumentation = OpenTelemetry::Instrumentation::Rack::Instrumentation.instance
      expect(rack_instrumentation).to be_installed

      config = rack_instrumentation.instance_variable_get(:@config)
      allowed_request_headers = config[:allowed_request_headers] || []
      sensitive_headers = %w[authorization cookie set-cookie x-forwarded-for]

      sensitive_headers.each do |header|
        msg = "Rack instrumentation should not record sensitive header: #{header}"
        expect(allowed_request_headers.map(&:downcase)).not_to include(header), msg
      end
    end
  end

  describe 'final exporter allowlist' do
    it 'does not register the old span-mutating sanitizer processor' do
      processors = OpenTelemetry.tracer_provider.instance_variable_get(:@span_processors)
      sanitizer_present = processors.any?(Otel::SpanSanitizingProcessor)

      expect(sanitizer_present).to be(false)
    end

    it 'does not allow tenant identifiers, record identifiers, or medication take payload fields' do
      safe_keys = Otel::AllowlistedSpanExporter::SAFE_ATTRIBUTE_KEYS

      expect(safe_keys).not_to include(
        'model.id',
        'account.id',
        'household.id',
        'person.id',
        'medication_take.dose_amount',
        'medication_take.taken_at',
        'db.statement'
      )
    end
  end
end
