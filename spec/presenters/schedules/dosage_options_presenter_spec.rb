# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Schedules::DosageOptionsPresenter do
  let(:schedule) { Schedule.new(dose_amount: 200, dose_unit: 'mg') }
  let(:matching_dosage) do
    double(
      id: 1,
      amount: schedule.dose_amount,
      unit: 'mg',
      description: 'tablet',
      selection_key: '200|mg',
      option_value: '1'
    )
  end
  let(:duplicate_dosage) do
    double(
      id: 2,
      amount: schedule.dose_amount,
      unit: 'mg',
      description: 'capsule',
      selection_key: '200|mg',
      option_value: '2'
    )
  end
  let(:dosage_records) { instance_double(ActiveRecord::Relation, order: [matching_dosage, duplicate_dosage]) }
  let(:medication) do
    instance_double(
      Medication,
      id: 123,
      dosage_records: dosage_records,
      dose_options_payload: [{ 'amount' => '1' }]
    )
  end

  before do
    allow(schedule).to receive(:medication).and_return(medication)
  end

  describe '#selected_dosage_option' do
    it 'returns the dosage matching the schedule selection' do
      presenter = described_class.new(schedule: schedule)

      expect(presenter.selected_dosage_option).to eq(matching_dosage)
    end
  end

  describe '#duplicate_dose_selection_keys' do
    it 'memoizes duplicate selection key calculation' do
      presenter = described_class.new(schedule: schedule)

      2.times { presenter.duplicate_dose_selection_keys }

      expect(medication).to have_received(:dosage_records).once
      expect(presenter.duplicate_dose_selection_keys).to eq(['200|mg'])
    end
  end

  describe '#dosages' do
    it 'memoizes the ordered dosage records across public presenter calls' do
      presenter = described_class.new(schedule: schedule)

      presenter.dosages
      presenter.selected_dosage_option
      presenter.duplicate_dose_selection_keys
      presenter.dosage_dom_id(duplicate_dosage)

      expect(medication).to have_received(:dosage_records).once
    end

    it 'returns an empty list when the schedule has no medication' do
      presenter = described_class.new(schedule: Schedule.new)

      expect(presenter.dosages).to eq([])
    end
  end

  describe '#dosage_dom_id' do
    it 'adds a description suffix when selection keys are duplicated' do
      presenter = described_class.new(schedule: schedule)

      expect(presenter.dosage_dom_id(duplicate_dosage)).to eq('schedule_dose_option_200_mg_capsule')
    end
  end
end
