# frozen_string_literal: true

require 'rails_helper'

# Tested through MedicationTake, which includes OtelInstrumented.
RSpec.describe OtelInstrumented do
  let(:schedule) { create(:schedule) }

  describe '.otel_tracer' do
    it 'returns a tracer named after the model' do
      tracer = MedicationTake.otel_tracer
      expect(tracer).to respond_to(:in_span)
    end

    it 'memoizes the tracer instance' do
      first_call  = MedicationTake.otel_tracer
      second_call = MedicationTake.otel_tracer
      expect(first_call).to be(second_call)
    end
  end

  describe 'after-create callback (trace_create)' do
    it 'does not raise when a record is created' do
      expect { create(:medication_take, :for_schedule, schedule: schedule) }.not_to raise_error
    end
  end

  describe 'after-update callback (trace_update)' do
    it 'does not raise when a record is updated' do
      take = create(:medication_take, :for_schedule, schedule: schedule)
      expect { take.update!(dose_amount: take.dose_amount + 1) }.not_to raise_error
    end
  end

  describe 'after-destroy callback (trace_destroy)' do
    it 'does not raise when a record is destroyed' do
      take = create(:medication_take, :for_schedule, schedule: schedule)
      expect { take.destroy! }.not_to raise_error
    end
  end

  describe '#otel_span_attributes' do
    it 'returns only non-sensitive model metadata for medication takes' do
      take  = create(:medication_take, :for_schedule, schedule: schedule)
      attrs = take.send(:otel_span_attributes, 'create')

      expect(attrs).to eq(
        'model.name' => 'MedicationTake',
        'model.operation' => 'create'
      )
      sensitive_keys = %w[
        model.id medication_take.taken_at medication_take.dose_amount medication_take.dose_unit
        medication_take.schedule_id medication_take.person_medication_id medication_take.taken_from_medication_id
        medication_take.taken_from_location_id
      ]
      expect(attrs.keys).not_to include(*sensitive_keys)
    end
  end

  describe 'tracer error resilience' do
    it 'logs a warning and does not re-raise when the tracer fails' do
      take = build(:medication_take, :for_schedule, schedule: schedule)
      allow(MedicationTake.otel_tracer).to receive(:in_span).and_raise(StandardError, 'tracer unavailable')
      allow(Rails.logger).to receive(:warn)

      expect { take.save! }.not_to raise_error
      expect(Rails.logger).to have_received(:warn).with(/OpenTelemetry.*tracer unavailable/)
    end
  end
end
