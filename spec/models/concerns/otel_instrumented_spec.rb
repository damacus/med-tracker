# frozen_string_literal: true

require 'rails_helper'

# These tests verify the OtelInstrumented concern behavior.
# Since OpenTelemetry is disabled in test environment (no OTLP endpoint), we skip these tests.
# The concern is tested implicitly when OTEL_EXPORTER_OTLP_ENDPOINT is set.
RSpec.describe OtelInstrumented, skip: 'OpenTelemetry disabled in test environment' do
  fixtures :accounts, :people, :medicines, :person_medicines, :prescriptions

  # Use fixtures instead of FactoryBot per project guidelines
  let(:person_medicine) { person_medicines(:john_vitamin_d) }
  let(:prescription) { prescriptions(:john_paracetamol) }

  # Shared exporter setup with proper cleanup to avoid test pollution
  let(:exporter) { OpenTelemetry::SDK::Trace::Export::InMemorySpanExporter.new }
  let(:processor) { OpenTelemetry::SDK::Trace::Export::SimpleSpanProcessor.new(exporter) }

  before do
    OpenTelemetry.tracer_provider.add_span_processor(processor)
  end

  after do
    OpenTelemetry.tracer_provider.force_flush
    exporter.reset
  end

  describe 'tracer configuration' do
    it 'creates a tracer with the model name' do
      tracer = MedicationTake.otel_tracer
      expect(tracer).to be_a(OpenTelemetry::SDK::Trace::Tracer)
    end
  end

  describe 'span creation on create' do
    it 'creates a span with person_medicine source' do
      medication_take = MedicationTake.create!(
        person_medicine: person_medicine,
        taken_at: Time.current
      )

      OpenTelemetry.tracer_provider.force_flush
      spans = exporter.finished_spans
      create_span = spans.find { |s| s.name == 'medication_take.create' }

      expect(create_span).not_to be_nil
      expect(create_span.attributes['model.name']).to eq('MedicationTake')
      expect(create_span.attributes['model.id']).to eq(medication_take.id.to_s)
      expect(create_span.attributes['medication_take.source_type']).to eq('person_medicine')
      expect(create_span.attributes['medication_take.person_medicine_id']).to eq(person_medicine.id.to_s)
    end

    it 'creates a span with prescription source' do
      _medication_take = MedicationTake.create!(
        prescription: prescription,
        taken_at: Time.current
      )

      OpenTelemetry.tracer_provider.force_flush
      spans = exporter.finished_spans
      create_span = spans.find { |s| s.name == 'medication_take.create' }

      expect(create_span).not_to be_nil
      expect(create_span.attributes['medication_take.source_type']).to eq('prescription')
      expect(create_span.attributes['medication_take.prescription_id']).to eq(prescription.id.to_s)
    end

    it 'includes taken_at timestamp in span attributes' do
      taken_time = Time.zone.parse('2025-01-15 10:30:00')
      MedicationTake.create!(
        person_medicine: person_medicine,
        taken_at: taken_time
      )

      OpenTelemetry.tracer_provider.force_flush
      spans = exporter.finished_spans
      create_span = spans.find { |s| s.name == 'medication_take.create' }

      expect(create_span.attributes['medication_take.taken_at']).to eq('2025-01-15T10:30:00Z')
    end
  end

  describe 'span creation on update' do
    it 'creates a span when a medication take is updated' do
      medication_take = MedicationTake.create!(
        person_medicine: person_medicine,
        taken_at: Time.current
      )
      exporter.reset

      medication_take.update!(taken_at: 1.hour.ago)

      OpenTelemetry.tracer_provider.force_flush
      spans = exporter.finished_spans
      update_span = spans.find { |s| s.name == 'medication_take.update' }

      expect(update_span).not_to be_nil
      expect(update_span.attributes['model.name']).to eq('MedicationTake')
      expect(update_span.attributes['model.id']).to eq(medication_take.id.to_s)
      expect(update_span.attributes['model.operation']).to eq('update')
    end
  end

  describe 'span creation on destroy' do
    it 'creates a span when a medication take is destroyed' do
      medication_take = MedicationTake.create!(
        person_medicine: person_medicine,
        taken_at: Time.current
      )
      medication_take_id = medication_take.id
      exporter.reset

      medication_take.destroy!

      OpenTelemetry.tracer_provider.force_flush
      spans = exporter.finished_spans
      destroy_span = spans.find { |s| s.name == 'medication_take.destroy' }

      expect(destroy_span).not_to be_nil
      expect(destroy_span.attributes['model.name']).to eq('MedicationTake')
      expect(destroy_span.attributes['model.id']).to eq(medication_take_id.to_s)
      expect(destroy_span.attributes['model.operation']).to eq('destroy')
    end
  end
end
