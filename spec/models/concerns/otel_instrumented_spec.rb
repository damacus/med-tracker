# frozen_string_literal: true

require 'rails_helper'

# Tested through MedicationTake, which includes OtelInstrumented.
RSpec.describe OtelInstrumented do
  let(:schedule) { create(:schedule) }
  let(:take_attrs) do
    {
      schedule: schedule,
      taken_at: Time.current,
      dose_amount: schedule.dose_amount,
      dose_unit: schedule.dose_unit
    }
  end

  describe '.otel_tracer' do
    it 'returns a tracer named after the model' do
      tracer = MedicationTake.otel_tracer
      expect(tracer).to respond_to(:in_span)
    end

    it 'memoizes the tracer instance' do
      expect(MedicationTake.otel_tracer).to be(MedicationTake.otel_tracer)
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

  describe '#otel_span_attributes (default implementation)' do
    it 'returns a hash containing model.name, model.id, and model.operation' do
      take = create(:medication_take, :for_schedule, schedule: schedule)
      attrs = take.send(:otel_span_attributes, 'create')

      expect(attrs).to include(
        'model.name'      => 'MedicationTake',
        'model.id'        => take.id.to_s,
        'model.operation' => 'create'
      )
    end
  end

  describe 'tracer error resilience' do
    it 'logs a warning and does not re-raise when the tracer fails' do
      take = build(:medication_take, :for_schedule, schedule: schedule)
      allow(MedicationTake.otel_tracer).to receive(:in_span).and_raise(StandardError, 'tracer unavailable')

      expect(Rails.logger).to receive(:warn).with(/OpenTelemetry.*tracer unavailable/)
      expect { take.save! }.not_to raise_error
    end
  end
end
