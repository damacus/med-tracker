# frozen_string_literal: true

require 'rails_helper'
require 'otel/span_sanitizing_processor'

RSpec.describe Otel::SpanSanitizingProcessor do
  subject(:processor) { described_class.new }

  let(:fake_span_class) do
    Class.new do
      attr_reader :attributes

      def initialize(attributes)
        @attributes = attributes
      end

      def set_attribute(key, value)
        attributes[key] = value
      end
    end
  end

  describe '#on_start' do
    it 'sanitizes email addresses in span attributes' do
      span = fake_span_class.new({ 'user.email' => 'john@example.com' })

      processor.on_start(span, OpenTelemetry::Context.current)

      expect(span.attributes['user.email']).to eq('[REDACTED]')
    end

    it 'sanitizes sensitive header attributes' do
      attrs = { 'http.request.header.authorization' => 'Bearer secret-token-123' }
      span = fake_span_class.new(attrs)

      processor.on_start(span, OpenTelemetry::Context.current)

      expect(span.attributes['http.request.header.authorization']).to eq('[REDACTED]')
    end

    it 'preserves non-sensitive attributes' do
      attrs = {
        'model.name' => 'MedicationTake',
        'model.id' => '42',
        'model.operation' => 'create'
      }
      span = fake_span_class.new(attrs)

      processor.on_start(span, OpenTelemetry::Context.current)

      expect(span.attributes['model.name']).to eq('MedicationTake')
      expect(span.attributes['model.id']).to eq('42')
      expect(span.attributes['model.operation']).to eq('create')
    end

    it 'redacts IP addresses in attribute values' do
      span = fake_span_class.new({ 'client.address' => '192.168.1.100' })

      processor.on_start(span, OpenTelemetry::Context.current)

      expect(span.attributes['client.address']).to eq('[IP REDACTED]')
    end

    it 'emits a bounded warning without interrupting the traced operation' do
      span = fake_span_class.new({ 'user.email' => 'john@example.com' })
      allow(span).to receive(:set_attribute).and_raise(RuntimeError, 'private source data')
      allow(Observability::DiagnosticEvent).to receive(:emit)

      expect { processor.on_start(span, OpenTelemetry::Context.current) }.not_to raise_error
      expect(Observability::DiagnosticEvent).to have_received(:emit).with(
        component: :span_sanitizer,
        reason: :operation_failed,
        severity: :warn,
        error: instance_of(RuntimeError)
      )
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
