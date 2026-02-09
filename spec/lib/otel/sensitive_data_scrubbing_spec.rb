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

  describe 'SpanSanitizer processor' do
    it 'is registered as a span processor' do
      processors = OpenTelemetry.tracer_provider.instance_variable_get(:@span_processors)
      sanitizer_present = processors.any? do |p|
        p.is_a?(Otel::SpanSanitizingProcessor)
      end

      expect(sanitizer_present).to be(true),
                                   'SpanSanitizingProcessor should be registered as a span processor'
    end
  end
end
