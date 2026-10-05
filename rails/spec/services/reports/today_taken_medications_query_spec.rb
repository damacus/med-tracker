# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Reports::TodayTakenMedicationsQuery do
  subject(:result) { described_class.new(people: people).call }

  let(:people) { Person.where(id: [alice.id, beta.id]) }
  let(:alice) { create(:person, name: 'Alice Patient') }
  let(:beta) { create(:person, name: 'Beta Patient') }
  let(:outside_scope) { create(:person, name: 'Outside Patient') }

  describe '#call' do
    it 'returns scheduled medications taken today' do
      schedule = create(:schedule, person: alice, medication: create(:medication, name: 'Paracetamol'))
      create(:medication_take, :for_schedule, schedule: schedule, taken_at: Time.zone.today.noon)

      expect(result.first.medications.map(&:name)).to eq(['Paracetamol'])
    end

    it 'returns direct person medications taken today' do
      person_medication = create(:person_medication, person: alice, medication: create(:medication, name: 'Vitamin D'))
      create(
        :medication_take,
        :for_person_medication,
        person_medication: person_medication,
        taken_at: Time.zone.today.noon
      )

      expect(result.first.medications.map(&:name)).to eq(['Vitamin D'])
    end

    it 'excludes medications taken outside today' do
      yesterday_schedule = create(:schedule, person: alice, medication: create(:medication, name: 'Yesterday Med'))
      future_schedule = create(:schedule, person: alice, medication: create(:medication, name: 'Future Med'))
      create(:medication_take, :for_schedule, schedule: yesterday_schedule, taken_at: 1.day.ago)
      create(:medication_take, :for_schedule, schedule: future_schedule, taken_at: 1.day.from_now)

      expect(result).to be_empty
    end

    it 'excludes medications for people outside the passed scope' do
      schedule = create(:schedule, person: outside_scope, medication: create(:medication, name: 'Hidden Med'))
      create(:medication_take, :for_schedule, schedule: schedule, taken_at: Time.zone.today.noon)

      expect(result).to be_empty
    end

    it 'deduplicates repeated medications for the same person and sorts groups by person name' do
      beta_schedule = create(:schedule, person: beta, medication: create(:medication, name: 'Zinc'))
      alice_schedule = create(:schedule, person: alice, medication: create(:medication, name: 'Aspirin'))
      create(:medication_take, :for_schedule, schedule: beta_schedule, taken_at: Time.zone.today.noon)
      create(:medication_take, :for_schedule, schedule: alice_schedule, taken_at: Time.zone.today.noon)
      create(:medication_take, :for_schedule, schedule: alice_schedule, taken_at: Time.zone.today.noon + 1.hour)

      expect(result.map { |group| [group.person.name, group.medications.map(&:name)] }).to eq(
        [
          ['Alice Patient', ['Aspirin']],
          ['Beta Patient', ['Zinc']]
        ]
      )
    end
  end
end
