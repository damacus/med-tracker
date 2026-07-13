# frozen_string_literal: true

require 'rails_helper'

# Tested through MedicationTake, which includes OtelInstrumented.
RSpec.describe OtelInstrumented do
  let(:schedule) { create(:schedule) }
  let(:mutable_record_class) do
    stub_const('OtelMutableTestRecord', Class.new(ApplicationRecord) do
      self.table_name = 'locations'
      include OtelInstrumented
    end)
  end
  let(:mutable_record) do
    mutable_record_class.create!(
      name: 'Mutable telemetry record',
      household_id: create(:household).id
    )
  end

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
    it 'traces an update on a mutable instrumented record' do
      tracer = instance_double(OpenTelemetry::Trace::Tracer)
      span = instance_double(OpenTelemetry::Trace::Span, add_event: nil)
      allow(mutable_record_class).to receive(:otel_tracer).and_return(tracer)
      allow(tracer).to receive(:in_span).and_yield(span)

      mutable_record.update!(name: 'Updated telemetry record')

      expect(tracer).to have_received(:in_span).with(
        'otel_mutable_test_record.update',
        attributes: { 'model.name' => 'OtelMutableTestRecord', 'model.operation' => 'update' },
        kind: :internal
      )
    end
  end

  describe 'after-destroy callback (trace_destroy)' do
    it 'traces destruction of a mutable instrumented record' do
      tracer = instance_double(OpenTelemetry::Trace::Tracer)
      span = instance_double(OpenTelemetry::Trace::Span, add_event: nil)
      allow(mutable_record_class).to receive(:otel_tracer).and_return(tracer)
      allow(tracer).to receive(:in_span).and_yield(span)

      mutable_record.destroy!

      expect(tracer).to have_received(:in_span).with(
        'otel_mutable_test_record.destroy',
        attributes: { 'model.name' => 'OtelMutableTestRecord', 'model.operation' => 'destroy' },
        kind: :internal
      )
    end
  end

  describe '#otel_span_attributes' do
    let(:sensitive_attribute_keys) do
      %w[
        model.id
        medication_take.taken_at
        medication_take.dose_amount
        medication_take.dose_unit
        medication_take.schedule_id
        medication_take.person_medication_id
        medication_take.taken_from_medication_id
        medication_take.taken_from_location_id
      ]
    end

    it 'returns non-sensitive model metadata with a stable record correlation hash' do
      take  = create(:medication_take, :for_schedule, schedule: schedule)
      attrs = take.send(:otel_span_attributes, 'create')

      expect(attrs).to include(
        'model.name' => 'MedicationTake',
        'model.operation' => 'create',
        'model.id_hash' => match(/\A\h{64}\z/)
      )
      expect(attrs['model.id_hash']).to eq(take.send(:otel_span_attributes, 'update')['model.id_hash'])
      expect(attrs['model.id_hash']).not_to eq(take.id.to_s)
      expect(attrs.keys).not_to include(*sensitive_attribute_keys)
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
