# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Prescription do
  fixtures :prescriptions, :people, :medicines, :dosages

  describe 'active flag' do
    let(:prescription) { prescriptions(:john_paracetamol) }

    it 'is active by default' do
      new_prescription = described_class.new(
        person: people(:john),
        medicine: medicines(:paracetamol),
        dosage: dosages(:paracetamol_adult),
        start_date: Time.zone.today,
        end_date: Time.zone.today + 30.days
      )
      new_prescription.save
      expect(new_prescription.active).to be true
    end

    it 'can be set to inactive' do
      prescription.update(active: false)
      expect(prescription.active).to be false
    end

    it 'can be reactivated' do
      prescription.update(active: false)
      prescription.update(active: true)
      expect(prescription.active).to be true
    end
  end

  describe 'validations' do
    subject(:prescription) do
      described_class.new(
        person: person,
        medicine: medicine,
        dosage: dosage,
        start_date: Time.zone.today,
        end_date: Time.zone.today + 30.days
      )
    end

    let(:person) do
      Person.create!(
        name: 'Jane Doe',
        email: 'jane@example.com',
        date_of_birth: Date.new(1990, 1, 1)
      )
    end
    let(:medicine) do
      Medicine.create!(
        name: 'Lisinopril',
        current_supply: 50,
        stock: 50,
        reorder_threshold: 10
      )
    end
    let(:dosage) { Dosage.create!(medicine: medicine, amount: 10, unit: 'mg', frequency: 'daily') }

    it { is_expected.to validate_presence_of(:start_date) }
    it { is_expected.to validate_presence_of(:end_date) }

    it 'is invalid if end_date is before start_date' do
      prescription.end_date = prescription.start_date - 1.day
      expect(prescription).not_to be_valid
    end
  end

  describe 'associations' do
    it { is_expected.to belong_to(:person) }
    it { is_expected.to belong_to(:dosage) }
    it { is_expected.to have_many(:medication_takes).dependent(:destroy) }
  end
end
