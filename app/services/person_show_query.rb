# frozen_string_literal: true

class PersonShowQuery
  Result = Data.define(:person, :schedules, :person_medications)

  attr_reader :person

  def initialize(person:)
    @person = person
  end

  def call
    pause_context = {
      medication_pause_periods: [{ recorded_by_membership: :person }, { resumed_by_membership: :person }]
    }
    schedules = person.schedules.current.includes(:medication, pause_context)
    person_medications = person.person_medications.current.includes(:medication, pause_context).ordered

    Result.new(
      person: person,
      schedules: schedules,
      person_medications: person_medications
    )
  end
end
