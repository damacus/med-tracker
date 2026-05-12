# frozen_string_literal: true

class PersonShowQuery
  Result = Data.define(:person, :schedules, :person_medications)

  attr_reader :person

  def initialize(person:)
    @person = person
  end

  def call
    schedules = person.schedules.includes(:medication)
    person_medications = person.person_medications.includes(:medication).ordered

    Result.new(
      person: person,
      schedules: schedules,
      person_medications: person_medications
    )
  end
end
