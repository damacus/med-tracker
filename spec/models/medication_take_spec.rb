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

  describe 'versioning' do # rubocop:disable RSpec/MultipleMemoizedHelpers
    fixtures :accounts, :people, :users

    let(:admin) { users(:admin) }
    let(:prescription) do
      person = people(:john)
      medicine = Medicine.create!(
        name: 'Test Medicine',
        current_supply: 100,
        stock: 100,
        reorder_threshold: 10
      )
      dosage = Dosage.create!(medicine: medicine, amount: 10, unit: 'mg', frequency: 'daily')

      Prescription.create!(
        person: person,
        medicine: medicine,
        dosage: dosage,
        start_date: Time.zone.today,
        end_date: Time.zone.today + 30.days
      )
    end

    before do
      PaperTrail.request.whodunnit = admin.id
    end

    after do
      PaperTrail.request.whodunnit = nil
    end

    it 'creates version when medication is taken' do
      expect do
        described_class.create!(
          prescription: prescription,
          taken_at: Time.current,
          amount_ml: 5.0
        )
      end.to change(PaperTrail::Version, :count).by(1)

      version = PaperTrail::Version.last
      expect(version.event).to eq('create')
      expect(version.item_type).to eq('MedicationTake')
    end

    it 'creates version on medication take update' do
      take = described_class.create!(
        prescription: prescription,
        taken_at: Time.current,
        amount_ml: 5.0
      )

      expect do
        take.update!(amount_ml: 10.0)
      end.to change(PaperTrail::Version, :count).by(1)

      version = take.versions.last
      expect(version.event).to eq('update')
      expect(version.object).to be_present
    end

    it 'tracks time changes for medication takes' do
      original_time = 2.hours.ago
      take = described_class.create!(
        prescription: prescription,
        taken_at: original_time,
        amount_ml: 5.0
      )

      new_time = 1.hour.ago
      take.update!(taken_at: new_time)

      version = take.versions.last
      reified = version.reify
      expect(reified.taken_at.to_i).to eq(original_time.to_i)
    end

    it 'associates version with current user' do
      take = described_class.create!(
        prescription: prescription,
        taken_at: Time.current,
        amount_ml: 5.0
      )
      expect(take.versions.last.whodunnit).to eq(admin.id.to_s)
    end
  end # rubocop:enable RSpec/MultipleMemoizedHelpers
end
