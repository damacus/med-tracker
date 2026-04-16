# frozen_string_literal: true

class PersonShowQuery
  Result = Data.define(:person, :schedules, :person_medications, :preloaded_takes)

  attr_reader :person

  def initialize(person:)
    @person = person
  end

  def call
    schedules = person.schedules.includes(:medication)
    person_medications = person.person_medications.includes(:medication).ordered
    preloaded_takes = {
      schedules: takes_by(schedules, :schedule_id),
      person_medications: takes_by(person_medications, :person_medication_id)
    }

    Result.new(
      person: person,
      schedules: schedules,
      person_medications: person_medications,
      preloaded_takes: preloaded_takes
    )
  end

  private

  def takes_by(records, foreign_key)
    MedicationTake
      .where(foreign_key => records.map(&:id), taken_at: today_range)
      .includes(:taken_from_location, :taken_from_medication)
      .order(taken_at: :desc)
      .group_by(&foreign_key)
  end

  def today_range
    Time.current.all_day
  end
end
