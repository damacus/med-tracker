# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Schedule do
  fixtures :accounts, :schedules, :people, :locations, :medications, :dosages

  describe 'active flag' do
    let(:schedule) { schedules(:john_paracetamol) }

    it 'is active by default' do
      new_schedule = described_class.new(
        person: people(:john),
        medication: medications(:paracetamol),
        dosage: dosages(:paracetamol_adult),
        start_date: Time.zone.today,
        end_date: Time.zone.today + 30.days
      )
      new_schedule.save
      expect(new_schedule.active).to be true
    end

    it 'can be set to inactive' do
      schedule.update(active: false)
      expect(schedule.active).to be false
    end

    it 'can be reactivated' do
      schedule.update(active: false)
      schedule.update(active: true)
      expect(schedule.active).to be true
    end
  end

  describe 'validations' do
    subject(:schedule) do
      described_class.new(
        person: person,
        medication: medication,
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
    let(:location) { Location.create!(name: 'Schedule Test Home') }
    let(:medication) do
      Medication.create!(
        name: 'Lisinopril',
        location: location,
        current_supply: 50,
        reorder_threshold: 10
      )
    end
    let(:dosage) { Dosage.create!(medication: medication, amount: 10, unit: 'mg', frequency: 'daily') }

    it { is_expected.to validate_presence_of(:start_date) }
    it { is_expected.to validate_presence_of(:end_date) }

    it 'is invalid if end_date is before start_date' do
      schedule.end_date = schedule.start_date - 1.day
      expect(schedule).not_to be_valid
    end
  end

  describe '#can_administer?' do
    let(:location) { Location.create!(name: 'Administer Test Home') }
    let(:medication) do
      Medication.create!(name: 'TestMed', location: location, current_supply: 10,
                         reorder_threshold: 2)
    end
    let(:person) do
      Person.create!(name: 'Test Person', email: 'test-administer@example.com', date_of_birth: Date.new(1990, 1, 1))
    end
    let(:dosage) { Dosage.create!(medication: medication, amount: 10, unit: 'mg', frequency: 'daily') }
    let(:schedule) do
      described_class.create!(
        person: person, medication: medication, dosage: dosage,
        start_date: Time.zone.today, end_date: Time.zone.today + 30.days
      )
    end

    context 'when can take now and medication in stock' do
      it 'returns true' do
        expect(schedule.can_administer?).to be true
      end
    end

    context 'when medication is out of stock' do
      before { medication.update!(current_supply: 0) }

      it 'returns false' do
        expect(schedule.can_administer?).to be false
      end
    end

    context 'when medication current_supply is nil (untracked)' do
      before { medication.update!(current_supply: nil) }

      it 'returns true' do
        expect(schedule.can_administer?).to be true
      end
    end
  end

  describe '#administration_blocked_reason' do
    let(:location) { Location.create!(name: 'Blocked Test Home') }
    let(:medication) do
      Medication.create!(name: 'TestMed2', location: location, current_supply: 0,
                         reorder_threshold: 2)
    end
    let(:person) do
      Person.create!(name: 'Test Person2', email: 'test-reason@example.com', date_of_birth: Date.new(1990, 1, 1))
    end
    let(:dosage) { Dosage.create!(medication: medication, amount: 10, unit: 'mg', frequency: 'daily') }
    let(:schedule) do
      described_class.create!(
        person: person, medication: medication, dosage: dosage,
        start_date: Time.zone.today, end_date: Time.zone.today + 30.days
      )
    end

    it 'returns :out_of_stock when medication has no supply' do
      expect(schedule.administration_blocked_reason).to eq(:out_of_stock)
    end
  end

  describe 'associations' do
    it { is_expected.to belong_to(:person) }
    it { is_expected.to belong_to(:dosage) }
    it { is_expected.to have_many(:medication_takes).dependent(:destroy) }
  end
end
