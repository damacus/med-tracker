# frozen_string_literal: true

require 'rails_helper'

RSpec.describe TakeMedicine do
  fixtures :prescriptions, :people, :medicines, :dosages

  describe 'validations' do
    subject(:take_medicine) do
      described_class.new(
        prescription: prescriptions(:john_paracetamol),
        taken_at: Time.current,
        amount_ml: 500.0
      )
    end

    it { is_expected.to validate_presence_of(:taken_at) }
    it { is_expected.to validate_presence_of(:amount_ml) }
    it { is_expected.to validate_numericality_of(:amount_ml).is_greater_than(0) }

    it 'is valid with valid attributes' do
      expect(take_medicine).to be_valid
    end

    it 'is invalid without amount_ml' do
      take_medicine.amount_ml = nil
      expect(take_medicine).not_to be_valid
    end

    it 'is invalid with zero amount_ml' do
      take_medicine.amount_ml = 0
      expect(take_medicine).not_to be_valid
    end

    it 'is invalid with negative amount_ml' do
      take_medicine.amount_ml = -5
      expect(take_medicine).not_to be_valid
    end
  end

  describe 'associations' do
    it { is_expected.to belong_to(:prescription) }
  end

  describe 'storing amount_ml' do
    it 'saves the amount_ml value' do
      take = described_class.create!(
        prescription: prescriptions(:john_paracetamol),
        taken_at: Time.current,
        amount_ml: 750.5
      )

      expect(take.reload.amount_ml).to eq(750.5)
    end
  end
end
