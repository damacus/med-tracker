# frozen_string_literal: true

require 'rails_helper'
require 'otel/span_sanitizing_processor'

RSpec.describe Otel::SpanSanitizingProcessor do
  subject(:processor) { described_class.new }

  let(:tracer) do
    OpenTelemetry.tracer_provider.tracer('test-tracer', '1.0.0')
  end

  let(:exporter) { OpenTelemetry::SDK::Trace::Export::InMemorySpanExporter.new }
  let(:simple_processor) { OpenTelemetry::SDK::Trace::Export::SimpleSpanProcessor.new(exporter) }

  before do
    OpenTelemetry.tracer_provider.add_span_processor(simple_processor)
  end

  after do
    OpenTelemetry.tracer_provider.force_flush
    exporter.reset
  end

  describe '#on_start' do
    it 'sanitizes email addresses in span attributes' do
      tracer.in_span('test.operation', attributes: { 'user.email' => 'john@example.com' }) do |span|
        processor.on_start(span, OpenTelemetry::Context.current)
        expect(span.attributes['user.email']).to eq('[REDACTED]')
      end
    end

    it 'sanitizes sensitive header attributes' do
      attrs = { 'http.request.header.authorization' => 'Bearer secret-token-123' }
      tracer.in_span('http.request', attributes: attrs) do |span|
        processor.on_start(span, OpenTelemetry::Context.current)
        expect(span.attributes['http.request.header.authorization']).to eq('[REDACTED]')
      end
    end

    it 'preserves non-sensitive attributes' do
      attrs = {
        'model.name' => 'MedicationTake',
        'model.id' => '42',
        'model.operation' => 'create'
      }
      tracer.in_span('medication_take.create', attributes: attrs) do |span|
        processor.on_start(span, OpenTelemetry::Context.current)
        expect(span.attributes['model.name']).to eq('MedicationTake')
        expect(span.attributes['model.id']).to eq('42')
        expect(span.attributes['model.operation']).to eq('create')
      end
    end

    it 'redacts IP addresses in attribute values' do
      tracer.in_span('test', attributes: { 'client.address' => '192.168.1.100' }) do |span|
        processor.on_start(span, OpenTelemetry::Context.current)
        expect(span.attributes['client.address']).to eq('[IP REDACTED]')
      end
    end
  end

  describe '#on_finish' do
    it 'responds to on_finish' do
      expect(processor).to respond_to(:on_finish)
    end
  end

  describe '#force_flush' do
    it 'returns success' do
      expect(processor.force_flush).to eq(OpenTelemetry::SDK::Trace::Export::SUCCESS)
    end
  end

  describe '#shutdown' do
    it 'returns success' do
      expect(processor.shutdown).to eq(OpenTelemetry::SDK::Trace::Export::SUCCESS)
    end
  end
end
