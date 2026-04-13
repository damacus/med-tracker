# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Dosage do
  subject(:dosage) do
    described_class.new(
      medication: medication,
      amount: 500,
      unit: 'mg',
      frequency: 'daily',
      description: 'Take with water'
    )
  end

  let(:location) { Location.create!(name: 'Test Home') }

  let(:medication) do
    Medication.create!(
      name: 'Aspirin',
      location: location,
      current_supply: 100,
      reorder_threshold: 10
    )
  end

  describe 'validations' do
    it { is_expected.to validate_presence_of(:amount) }
    it { is_expected.to validate_presence_of(:unit) }
    it { is_expected.to validate_presence_of(:frequency) }
    it { is_expected.to validate_numericality_of(:amount).is_greater_than(0) }
  end

  describe 'associations' do
    it { is_expected.to belong_to(:medication) }
  end

  describe '#sync_medication_dosage' do
    let(:medication) { create(:medication, dosage_amount: 500, dosage_unit: 'mg') }

    it 'clears the standard dosage amount while preserving the medication unit' do
      expect do
        create(:dosage, medication: medication, amount: 10, unit: 'mg')
      end.to change { medication.reload.dosage_amount }.from(500).to(nil)
      expect(medication.reload.dosage_unit).to eq('mg')
    end
  end
end
