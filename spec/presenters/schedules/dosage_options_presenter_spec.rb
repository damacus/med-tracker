# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Schedules::DosageOptionsPresenter do
  let(:schedule) { Schedule.new(dose_amount: 200, dose_unit: 'mg') }
  let(:matching_dosage) do
    double(
      amount: schedule.dose_amount,
      unit: 'mg',
      description: 'tablet',
      selection_key: '200|mg'
    )
  end
  let(:duplicate_dosage) do
    double(
      amount: schedule.dose_amount,
      unit: 'mg',
      description: 'capsule',
      selection_key: '200|mg'
    )
  end
  let(:medication) do
    instance_double(
      Medication,
      id: 123,
      dosages: [matching_dosage, duplicate_dosage],
      dose_options_payload: [{ 'amount' => '1' }]
    )
  end

  before do
    allow(schedule).to receive(:medication).and_return(medication)
  end

  describe '#selected_dosage_option' do
    it 'returns the dosage matching the schedule selection' do
      presenter = described_class.new(schedule: schedule, medications: [medication])

      expect(presenter.selected_dosage_option).to eq(matching_dosage)
    end
  end

  describe '#duplicate_dose_selection_keys' do
    it 'memoizes duplicate selection key calculation' do
      presenter = described_class.new(schedule: schedule, medications: [medication])

      2.times { presenter.duplicate_dose_selection_keys }

      expect(medication).to have_received(:dosages).once
      expect(presenter.duplicate_dose_selection_keys).to eq(['200|mg'])
    end
  end

  describe '#dosage_dom_id' do
    it 'adds a description suffix when selection keys are duplicated' do
      presenter = described_class.new(schedule: schedule, medications: [medication])

      expect(presenter.dosage_dom_id(duplicate_dosage)).to eq('schedule_dose_option_200_mg_capsule')
    end
  end

  describe '#medication_dose_options' do
    it 'maps medications to their dose option payloads' do
      presenter = described_class.new(schedule: schedule, medications: [medication])

      expect(presenter.medication_dose_options).to eq('123' => [{ 'amount' => '1' }])
    end
  end
end
