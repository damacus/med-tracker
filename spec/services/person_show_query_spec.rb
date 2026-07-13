# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PersonShowQuery do
  describe '#call' do
    it 'returns the person show read model for one person' do
      person = create(:person)
      other_person = create(:person)
      show_data = create_show_data_for(person: person, other_person: other_person)

      result = described_class.new(person: person).call

      expect(result.person).to eq(person)
      expect(result.schedules).to contain_exactly(show_data[:schedule])
      expect(result.person_medications).to contain_exactly(show_data[:person_medication])
      expect(result.members).to contain_exactly(:person, :schedules, :person_medications)
    end

    it 'omits retired administration sources' do
      person = create(:person)
      schedule = create(:schedule, person: person)
      person_medication = create(:person_medication, person: person)
      schedule.retire!
      person_medication.retire!

      result = described_class.new(person: person).call

      expect(result.schedules).to be_empty
      expect(result.person_medications).to be_empty
    end
  end

  def create_show_data_for(person:, other_person:)
    schedule = create(:schedule, person: person)
    create(:schedule, person: other_person)
    person_medication = create(:person_medication, person: person)
    create(:person_medication, person: other_person)

    {
      schedule: schedule,
      person_medication: person_medication
    }
  end
end
