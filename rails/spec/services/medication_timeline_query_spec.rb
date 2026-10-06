# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationTimelineQuery do
  describe '#call' do
    it 'returns related timeline sources for one medication and excludes the passed source' do
      medication = create(:medication)
      other_medication = create(:medication)
      timeline_data = create_timeline_data_for(medication: medication, other_medication: other_medication)

      result = described_class.new(medication: medication, excluding: timeline_data[:schedule]).call

      expect(result.schedules).to contain_exactly(timeline_data[:other_schedule])
      expect(result.person_medications).to contain_exactly(timeline_data[:person_medication])
    end

    it 'excludes paused timeline sources' do
      medication = create(:medication)
      paused_schedule = create(:schedule, medication: medication, active: false)
      paused_person_medication = create(:person_medication, medication: medication, active: false)

      result = described_class.new(medication: medication).call

      expect(result.schedules).not_to include(paused_schedule)
      expect(result.person_medications).not_to include(paused_person_medication)
    end

    it 'limits results to provided authorization scopes' do
      medication = create(:medication)
      authorized_person = create(:person)
      unauthorized_person = create(:person)
      authorized_schedule = create(:schedule, person: authorized_person, medication: medication)
      authorized_person_medication = create(:person_medication, person: authorized_person, medication: medication)
      create(:schedule, person: unauthorized_person, medication: medication)
      create(:person_medication, person: unauthorized_person, medication: medication)

      result = described_class.new(
        medication: medication,
        schedules_scope: Schedule.where(person: authorized_person),
        person_medications_scope: PersonMedication.where(person: authorized_person)
      ).call

      expect(result.schedules).to contain_exactly(authorized_schedule)
      expect(result.person_medications).to contain_exactly(authorized_person_medication)
    end
  end

  def create_timeline_data_for(medication:, other_medication:)
    schedule = create(:schedule, medication: medication)
    other_schedule = create(:schedule, medication: medication)
    create(:schedule, :expired, medication: medication)
    create(:schedule, medication: other_medication)
    person_medication = create(:person_medication, medication: medication)
    create(:person_medication, medication: other_medication)

    {
      schedule: schedule,
      other_schedule: other_schedule,
      person_medication: person_medication
    }
  end
end
