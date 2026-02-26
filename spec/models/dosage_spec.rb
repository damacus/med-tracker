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
    it { is_expected.to have_many(:schedules).dependent(:destroy) }
  end
end
