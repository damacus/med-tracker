# frozen_string_literal: true

require 'rails_helper'

RSpec.describe OtelInstrumented do
  # Use MedicationTake as the test subject since it includes the concern
  let(:person_medicine) { create(:person_medicine) }

  describe 'tracer configuration' do
    it 'creates a tracer with the model name' do
      tracer = MedicationTake.otel_tracer
      expect(tracer).to be_a(OpenTelemetry::SDK::Trace::Tracer)
    end
  end

  describe 'span creation on create' do
    it 'creates a span when a medication take is created' do
      # Use an in-memory exporter to capture spans
      exporter = OpenTelemetry::SDK::Trace::Export::InMemorySpanExporter.new
      processor = OpenTelemetry::SDK::Trace::Export::SimpleSpanProcessor.new(exporter)
      OpenTelemetry.tracer_provider.add_span_processor(processor)

      medication_take = MedicationTake.create!(
        person_medicine: person_medicine,
        taken_at: Time.current
      )

      # Force flush to ensure spans are exported
      OpenTelemetry.tracer_provider.force_flush

      spans = exporter.finished_spans
      create_span = spans.find { |s| s.name == 'medication_take.create' }

      expect(create_span).not_to be_nil
      expect(create_span.attributes['model.name']).to eq('MedicationTake')
      expect(create_span.attributes['model.id']).to eq(medication_take.id.to_s)
      expect(create_span.attributes['medication_take.source_type']).to eq('person_medicine')
      expect(create_span.attributes['medication_take.person_medicine_id']).to eq(person_medicine.id.to_s)
    end
  end

  describe 'custom span attributes' do
    let(:medication_take) do
      MedicationTake.new(
        person_medicine: person_medicine,
        taken_at: Time.zone.parse('2025-01-15 10:30:00')
      )
    end

    it 'includes medication_take.taken_at attribute' do
      medication_take.save!
      attrs = medication_take.send(:otel_span_attributes, 'create')

      expect(attrs['medication_take.taken_at']).to eq('2025-01-15T10:30:00Z')
    end

    it 'includes person_medicine source attributes when person_medicine is set' do
      medication_take.save!
      attrs = medication_take.send(:otel_span_attributes, 'create')

      expect(attrs['medication_take.source_type']).to eq('person_medicine')
      expect(attrs['medication_take.person_medicine_id']).to eq(person_medicine.id.to_s)
    end
  end
end
