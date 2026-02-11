# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Prescription do
  fixtures :accounts, :prescriptions, :people, :medicines, :dosages

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

  describe '#can_administer?' do
    let(:medicine) { Medicine.create!(name: 'TestMed', stock: 10, reorder_threshold: 2) }
    let(:person) do
      Person.create!(name: 'Test Person', email: 'test-administer@example.com', date_of_birth: Date.new(1990, 1, 1))
    end
    let(:dosage) { Dosage.create!(medicine: medicine, amount: 10, unit: 'mg', frequency: 'daily') }
    let(:prescription) do
      described_class.create!(
        person: person, medicine: medicine, dosage: dosage,
        start_date: Time.zone.today, end_date: Time.zone.today + 30.days
      )
    end

    context 'when can take now and medicine in stock' do
      it 'returns true' do
        expect(prescription.can_administer?).to be true
      end
    end

    context 'when medicine is out of stock' do
      before { medicine.update!(stock: 0) }

      it 'returns false' do
        expect(prescription.can_administer?).to be false
      end
    end

    context 'when medicine stock is nil (untracked)' do
      before { medicine.update!(stock: nil) }

      it 'returns true' do
        expect(prescription.can_administer?).to be true
      end
    end
  end

  describe '#administration_blocked_reason' do
    let(:medicine) { Medicine.create!(name: 'TestMed2', stock: 0, reorder_threshold: 2) }
    let(:person) do
      Person.create!(name: 'Test Person2', email: 'test-reason@example.com', date_of_birth: Date.new(1990, 1, 1))
    end
    let(:dosage) { Dosage.create!(medicine: medicine, amount: 10, unit: 'mg', frequency: 'daily') }
    let(:prescription) do
      described_class.create!(
        person: person, medicine: medicine, dosage: dosage,
        start_date: Time.zone.today, end_date: Time.zone.today + 30.days
      )
    end

    it 'returns :out_of_stock when medicine has no stock' do
      expect(prescription.administration_blocked_reason).to eq(:out_of_stock)
    end
  end

  describe 'associations' do
    it { is_expected.to belong_to(:person) }
    it { is_expected.to belong_to(:dosage) }
    it { is_expected.to have_many(:medication_takes).dependent(:destroy) }
  end
end
