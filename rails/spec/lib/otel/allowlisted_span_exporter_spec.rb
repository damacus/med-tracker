# frozen_string_literal: true

require 'rails_helper'
require 'opentelemetry/exporter/otlp'
require 'otel/allowlisted_span_exporter'

RSpec.describe Otel::AllowlistedSpanExporter do
  let(:fake_span_class) { Struct.new(:name, :attributes, :resource) }
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

  it 'sanitizes the complete exported span surface' do
    marker = 'Daniel Webb takes Wellman Original at 08:04'
    exporter.export([complete_span(marker)])
    exported = fake_exporter.exported.sole
    expect_complete_surface_sanitized(exported, marker)
  end

  it 'drops Active Job arguments from the final exporter surface' do
    job_span = fake_span_class.new(
      'MissedDoseNotificationJob',
      {
        'job.system' => 'active_job',
        'job.arguments' => '[42,43,"2026-05-12","07:15"]',
        'messaging.message.body' => 'Daniel may have missed a dose',
        'error.type' => 'RuntimeError'
      }
    )

    exporter.export([job_span])

    expect(fake_exporter.exported.sole.attributes).to eq('error.type' => 'RuntimeError')
  end

  it 'preserves host.name from attribute-enumerator resources without mutating the source resource' do
    resource = OpenTelemetry::SDK::Resources::Resource.create(
      'host.name' => 'med-tracker-canary-7d5c8d7c8b-abcde',
      'process.pid' => 42,
      'unsafe.resource' => 'private-value'
    )

    expect_allowlisted_resource(resource, 'med-tracker-canary-7d5c8d7c8b-abcde')
  end

  it 'preserves host.name from attribute resources without mutating the source resource' do
    resource = Struct.new(:attributes).new(
      {
        'host.name' => 'med-tracker-canary-7d5c8d7c8b-fghij',
        'process.pid' => 43,
        'unsafe.resource' => 'private-value'
      }
    )

    expect_allowlisted_resource(resource, 'med-tracker-canary-7d5c8d7c8b-fghij')
  end

  it 'preserves the OTLP encoding contract for SDK span data' do
    marker = 'private-exporter-marker'
    provider, spans = sdk_span_data(marker)
    otlp_exporter = encoding_exporter(marker)

    result = described_class.new(otlp_exporter).export(spans)

    expect(result).to eq(OpenTelemetry::SDK::Trace::Export::SUCCESS)
  ensure
    provider&.shutdown
  end

  def sdk_span_data(marker)
    capture_exporter = OpenTelemetry::SDK::Trace::Export::InMemorySpanExporter.new
    resource = OpenTelemetry::SDK::Resources::Resource.create('unsafe.resource' => marker)
    provider = OpenTelemetry::SDK::Trace::TracerProvider.new(resource:)
    provider.add_span_processor(OpenTelemetry::SDK::Trace::Export::SimpleSpanProcessor.new(capture_exporter))
    provider.tracer('allowlisted-exporter-spec').in_span(
      'medication_take.characterization', links: [sdk_link(marker)]
    ) do |span|
      span.set_attribute('unsafe.attribute', marker)
      span.add_event(marker, attributes: { 'unsafe.event' => marker })
      span.status = OpenTelemetry::Trace::Status.error(marker)
    end
    [provider, capture_exporter.finished_spans]
  end

  def sdk_link(marker)
    context = OpenTelemetry::Trace::SpanContext.new(
      trace_id: "\x01" * 16,
      span_id: "\x02" * 8,
      trace_flags: OpenTelemetry::Trace::TraceFlags::SAMPLED,
      remote: true
    )
    OpenTelemetry::Trace::Link.new(context, 'unsafe.link' => marker)
  end

  def encoding_exporter(marker)
    exporter = OpenTelemetry::Exporter::OTLP::Exporter.new(
      endpoint: 'http://localhost:4318/v1/traces', compression: 'none'
    )
    allow(exporter).to receive(:send_bytes) do |bytes, **|
      expect(bytes).not_to include(marker)
      OpenTelemetry::SDK::Trace::Export::SUCCESS
    end
    exporter
  end

  def complete_span(marker)
    event = Struct.new(:name, :attributes).new(marker, { 'unsafe.event' => marker })
    status = Struct.new(:code, :description).new(:error, marker)
    link = Struct.new(:span_context, :attributes).new('opaque-context', { 'unsafe.link' => marker })
    resource = Struct.new(:attributes).new({ 'unsafe.resource' => marker })
    Struct.new(:name, :attributes, :events, :status, :links, :resource, keyword_init: true).new(
      name: marker,
      attributes: { 'model.name' => 'MedicationTake', 'unsafe.attribute' => marker },
      events: [event],
      status:,
      links: [link],
      resource:
    )
  end

  def expect_complete_surface_sanitized(exported, marker)
    expect(exported.to_json).not_to include(marker)
    expect_exported_surface_empty(exported)
  end

  def expect_allowlisted_resource(resource, host_name)
    source_attributes = resource_attributes(resource).dup
    exported_resource = export_resource(resource)

    expect(resource_attributes(exported_resource)).to eq('host.name' => host_name)
    expect(exported_resource).not_to equal(resource)
    expect(resource_attributes(resource)).to eq(source_attributes)
  end

  def export_resource(resource)
    exporter.export([fake_span_class.new('medication_take.create', {}, resource)])
    fake_exporter.exported.sole.resource
  end

  def resource_attributes(resource)
    return resource.attribute_enumerator.to_h if resource.respond_to?(:attribute_enumerator)

    resource.attributes
  end

  def expect_exported_surface_empty(exported)
    expect(exported).to have_attributes(
      name: 'span.filtered',
      events: be_empty,
      status: have_attributes(description: nil),
      links: contain_exactly(have_attributes(attributes: be_empty)),
      resource: have_attributes(attributes: be_empty)
    )
  end
end
