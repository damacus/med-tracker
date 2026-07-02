# frozen_string_literal: true

require 'rails_helper'
require 'otel/allowlisted_span_exporter'

RSpec.describe Otel::AllowlistedSpanExporter do
  let(:fake_span_class) { Struct.new(:name, :attributes) }
  let(:fake_exporter_class) do
    Class.new do
      attr_reader :exported

      def export(span_data, timeout: nil)
        @exported = span_data
        @timeout = timeout
        OpenTelemetry::SDK::Trace::Export::SUCCESS
      end

      def force_flush(timeout: nil)
        @force_flush_timeout = timeout
        OpenTelemetry::SDK::Trace::Export::SUCCESS
      end

      def shutdown(timeout: nil)
        @shutdown_timeout = timeout
        OpenTelemetry::SDK::Trace::Export::SUCCESS
      end
    end
  end
  let(:fake_exporter) { fake_exporter_class.new }
  let(:exporter) { described_class.new(fake_exporter) }
  let(:span) do
    fake_span_class.new(
      'medication_take.create',
      {
        'model.name' => 'MedicationTake',
        'model.operation' => 'create',
        'model.id' => '123',
        'model.id_hash' => '169e81c4d785338b1599a3af36a71fd7c21bbfb3ab7c8df5b74d9f678d5355e8',
        'medication_take.dose_amount' => '10',
        'medication_take.taken_at' => '2026-06-22T10:00:00Z',
        'account.id' => '99',
        'db.system' => 'postgresql',
        'db.statement' => 'SELECT * FROM people',
        'error.type' => 'RuntimeError',
        'exception.escaped' => true,
        'exception.source' => 'request',
        'exception.message' => 'person name leaked'
      }
    )
  end

  it 'exports copied spans with only allowlisted attributes' do
    expect(exporter.export([span])).to eq(OpenTelemetry::SDK::Trace::Export::SUCCESS)

    exported_span = fake_exporter.exported.sole
    expect(exported_span).not_to equal(span)
    expect(exported_span.attributes).to eq(
      'model.name' => 'MedicationTake',
      'model.operation' => 'create',
      'model.id_hash' => '169e81c4d785338b1599a3af36a71fd7c21bbfb3ab7c8df5b74d9f678d5355e8',
      'db.system' => 'postgresql',
      'error.type' => 'RuntimeError',
      'exception.escaped' => true,
      'exception.source' => 'request'
    )
    expect(span.attributes).to include('model.id' => '123')
  end
end
