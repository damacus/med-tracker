# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationTake do
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

  let(:person_medicine) do
    PersonMedicine.create!(
      person: person,
      medicine: medicine,
      notes: 'Test notes'
    )
  end

  describe 'validations' do
    it { is_expected.to validate_presence_of(:taken_at) }
  end

  describe 'associations' do
    it { is_expected.to belong_to(:prescription).optional }
    it { is_expected.to belong_to(:person_medicine).optional }
  end

  describe 'source validation' do
    context 'when neither prescription nor person_medicine is set' do
      subject(:medication_take) { described_class.new(taken_at: Time.current) }

      it 'is invalid' do
        expect(medication_take).not_to be_valid
        expect(medication_take.errors[:base]).to include(
          'Must have exactly one source (prescription or person_medicine)'
        )
      end
    end

    context 'when both prescription and person_medicine are set' do
      subject(:medication_take) do
        described_class.new(
          prescription: prescription,
          person_medicine: person_medicine,
          taken_at: Time.current
        )
      end

      it 'is invalid' do
        expect(medication_take).not_to be_valid
        expect(medication_take.errors[:base]).to include(
          'Must have exactly one source (prescription or person_medicine)'
        )
      end
    end

    context 'when only prescription is set' do
      subject(:medication_take) do
        described_class.new(
          prescription: prescription,
          taken_at: Time.current
        )
      end

      it 'is valid' do
        expect(medication_take).to be_valid
      end
    end

    context 'when only person_medicine is set' do
      subject(:medication_take) do
        described_class.new(
          person_medicine: person_medicine,
          taken_at: Time.current
        )
      end

      let(:person_medicine) do
        PersonMedicine.create!(
          person: person,
          medicine: medicine
        )
      end

      it 'is valid' do
        expect(medication_take).to be_valid
      end
    end
  end
end
