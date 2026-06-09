# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationDailyConsumption do
  subject(:consumption) { described_class.new(medication) }

  let(:schedules_proxy) { instance_double(ActiveRecord::Associations::CollectionProxy) }
  let(:medication) do
    instance_double(Medication, schedules: schedules_proxy, person_medications: [])
  end

  before do
    allow(schedules_proxy).to receive(:select).and_return([])
  end

  describe '#call' do
    context 'when there are no active schedules and no person_medications' do
      it 'returns 0.0' do
        expect(consumption.call).to eq(0.0)
      end
    end

    context 'when there is one active schedule with a daily rate' do
      let(:schedule) do
        instance_double(
          Schedule,
          active?: true,
          max_daily_doses: 2,
          cycle_period: 1.day
        )
      end

      before do
        allow(schedules_proxy).to receive(:select).and_yield(schedule).and_return([schedule])
        allow(schedule).to receive(:effective_dose_amount).with(Time.zone.today).and_return(1)
        allow(schedule).to receive(:effective_dose_unit).with(Time.zone.today).and_return('tablet')
        allow(MedicationStockConsumption).to receive(:quantity_for)
          .with(dose_amount: 1, dose_unit: 'tablet')
          .and_return(1)
      end

      it 'returns the daily rate for that schedule' do
        expect(consumption.call).to eq(2.0)
      end
    end

    context 'when a schedule has blank max_daily_doses' do
      let(:schedule) do
        instance_double(Schedule, active?: true, max_daily_doses: nil)
      end

      before do
        allow(schedules_proxy).to receive(:select).and_return([schedule])
      end

      it 'skips schedules with no max_daily_doses' do
        expect(consumption.call).to eq(0.0)
      end
    end

    context 'when there is a person_medication with a daily rate' do
      let(:person_medication) do
        instance_double(PersonMedication, max_daily_doses: 3, default_dose_amount: 2, dose_unit: 'ml')
      end

      before do
        allow(medication).to receive(:person_medications).and_return([person_medication])
        allow(MedicationStockConsumption).to receive(:quantity_for)
          .with(dose_amount: 2, dose_unit: 'ml')
          .and_return(2)
      end

      it 'includes person_medication rate in the total' do
        expect(consumption.call).to eq(6.0)
      end
    end
  end
end
