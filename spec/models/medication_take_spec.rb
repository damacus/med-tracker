# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationTake do
  subject(:medication_take) { described_class.new(schedule: schedule, taken_at: Time.current) }

  let(:person) { Person.create!(name: 'Jane Doe', date_of_birth: '1990-01-01') }

  let(:medication) do
    Medication.create!(
      name: 'Lisinopril',
      location: Location.find_or_create_by!(name: 'Test Home'),
      current_supply: 50,
      reorder_threshold: 10
    )
  end

  let(:dosage) { Dosage.create!(medication: medication, amount: 10, unit: 'mg', frequency: 'daily') }

  let(:schedule) do
    Schedule.create!(
      person: person,
      medication: medication,
      dosage: dosage,
      start_date: Time.zone.today,
      end_date: Time.zone.today + 30.days
    )
  end

  let(:person_medication) do
    PersonMedication.create!(
      person: person,
      medication: medication,
      notes: 'Test notes'
    )
  end

  describe 'validations' do
    it { is_expected.to validate_presence_of(:taken_at) }
  end

  describe 'associations' do
    it { is_expected.to belong_to(:schedule).optional }
    it { is_expected.to belong_to(:person_medication).optional }
  end

  describe 'source validation' do
    context 'when neither schedule nor person_medication is set' do
      subject(:medication_take) { described_class.new(taken_at: Time.current) }

      it 'is invalid' do
        expect(medication_take).not_to be_valid
        expect(medication_take.errors[:base]).to include(
          'Must have exactly one source (schedule or person_medication)'
        )
      end
    end

    context 'when both schedule and person_medication are set' do
      subject(:medication_take) do
        described_class.new(
          schedule: schedule,
          person_medication: person_medication,
          taken_at: Time.current
        )
      end

      it 'is invalid' do
        expect(medication_take).not_to be_valid
        expect(medication_take.errors[:base]).to include(
          'Must have exactly one source (schedule or person_medication)'
        )
      end
    end

    context 'when only schedule is set' do
      subject(:medication_take) do
        described_class.new(
          schedule: schedule,
          taken_at: Time.current
        )
      end

      it 'is valid' do
        expect(medication_take).to be_valid
      end
    end

    context 'when only person_medication is set' do
      subject(:medication_take) do
        described_class.new(
          person_medication: person_medication,
          taken_at: Time.current
        )
      end

      let(:person_medication) do
        PersonMedication.create!(
          person: person,
          medication: medication
        )
      end

      it 'is valid' do
        expect(medication_take).to be_valid
      end
    end
  end

  describe 'supply tracking' do
    before do
      medication.update!(current_supply: 100)
    end

    context 'when taking a dose from a schedule' do
      it 'deducts 1 from the medication current_supply' do
        expect do
          described_class.create!(
            schedule: schedule,
            taken_at: Time.current
          )
        end.to change { medication.reload.current_supply }.from(100).to(99)
      end
    end

    context 'when taking a dose from a person_medication' do
      it 'deducts 1 from the medication current_supply' do
        expect do
          described_class.create!(
            person_medication: person_medication,
            taken_at: Time.current
          )
        end.to change { medication.reload.current_supply }.from(100).to(99)
      end
    end
  end

  describe 'versioning' do # rubocop:disable RSpec/MultipleMemoizedHelpers
    fixtures :accounts, :people, :users

    let(:admin) { users(:admin) }
    let(:schedule) do
      person = people(:john)
      medication = Medication.create!(
        name: 'Test Medication',
        location: Location.find_or_create_by!(name: 'Versioning Home'),
        current_supply: 100,
        reorder_threshold: 10
      )
      dosage = Dosage.create!(medication: medication, amount: 10, unit: 'mg', frequency: 'daily')

      Schedule.create!(
        person: person,
        medication: medication,
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
          schedule: schedule,
          taken_at: Time.current,
          amount_ml: 5.0
        )
      end.to change(PaperTrail::Version.where(item_type: 'MedicationTake'), :count).by(1)

      version = PaperTrail::Version.where(item_type: 'MedicationTake').last
      expect(version.event).to eq('create')
      expect(version.item_type).to eq('MedicationTake')
    end

    it 'creates version on medication take update' do
      take = described_class.create!(
        schedule: schedule,
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
        schedule: schedule,
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
        schedule: schedule,
        taken_at: Time.current,
        amount_ml: 5.0
      )
      expect(take.versions.last.whodunnit).to eq(admin.id.to_s)
    end

    it 'records IP address when controller_info is set' do
      PaperTrail.request.controller_info = { ip: '192.168.1.100' }

      take = described_class.create!(
        schedule: schedule,
        taken_at: Time.current,
        amount_ml: 5.0
      )

      expect(take.versions.last.ip).to eq('192.168.1.100')
    ensure
      PaperTrail.request.controller_info = nil
    end
  end # rubocop:enable RSpec/MultipleMemoizedHelpers
end
