# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationTake, type: :model do
  subject(:medication_take) { described_class.new(prescription: prescription, taken_at: Time.current) }

  let(:person) { Person.create!(name: 'Jane Doe', date_of_birth: '1990-01-01') }

  let(:medicine) do
    Medicine.create!(
      name: 'Lisinopril',
      current_supply: 50,
      stock: 50,
      reorder_threshold: 10
    )
  end

  let(:dosage) { Dosage.create!(medicine: medicine, amount: 10, unit: 'mg', frequency: 'daily') }

  let(:prescription) do
    Prescription.create!(
      person: person,
      medicine: medicine,
      dosage: dosage,
      start_date: Time.zone.today,
      end_date: Time.zone.today + 30.days
    )
  end

  describe 'validations' do
    it { is_expected.to validate_presence_of(:taken_at) }
  end

  describe 'associations' do
    it { is_expected.to belong_to(:prescription) }
  end
end
