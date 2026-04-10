# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PersonShowQuery do
  describe '#call' do
    it 'returns the person-show read model for a single person' do
      person = create(:person)
      other_person = create(:person)
      show_data = create_show_data_for(person: person, other_person: other_person)

      result = described_class.new(person: person).call

      expect(result.person).to eq(person)
      expect(result.schedules).to contain_exactly(show_data[:schedule])
      expect(result.person_medications).to contain_exactly(show_data[:person_medication])
      expect(result.preloaded_takes[:schedules]).to eq(
        show_data[:schedule].id => [show_data[:todays_schedule_take]]
      )
      expect(result.preloaded_takes[:person_medications]).to eq(
        show_data[:person_medication].id => [show_data[:todays_person_medication_take]]
      )
    end
  end

  def create_show_data_for(person:, other_person:)
    schedule = create(:schedule, person: person)
    other_schedule = create(:schedule, person: other_person)
    person_medication = create(:person_medication, person: person)
    other_person_medication = create(:person_medication, person: other_person)

    todays_schedule_take = create_schedule_takes(schedule: schedule, other_schedule: other_schedule)
    todays_person_medication_take = create_person_medication_takes(
      person_medication: person_medication,
      other_person_medication: other_person_medication
    )

    {
      schedule: schedule,
      person_medication: person_medication,
      todays_schedule_take: todays_schedule_take,
      todays_person_medication_take: todays_person_medication_take
    }
  end

  def create_schedule_takes(schedule:, other_schedule:)
    todays_schedule_take = create(:medication_take, :for_schedule, :today, schedule: schedule)
    create(:medication_take, :for_schedule, taken_at: 2.days.ago, schedule: schedule)
    create(:medication_take, :for_schedule, :today, schedule: other_schedule)
    todays_schedule_take
  end

  def create_person_medication_takes(person_medication:, other_person_medication:)
    todays_person_medication_take = create(
      :medication_take,
      :for_person_medication,
      :today,
      person_medication: person_medication
    )
    create(:medication_take, :for_person_medication, taken_at: 2.days.ago, person_medication: person_medication)
    create(:medication_take, :for_person_medication, :today, person_medication: other_person_medication)
    todays_person_medication_take
  end
end
