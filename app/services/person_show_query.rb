# frozen_string_literal: true

class PersonShowQuery
  Result = Data.define(:person, :schedules, :person_medications, :preloaded_takes)

  attr_reader :person

  def initialize(person:)
    @person = person
  end

  def call
    schedules = person.schedules.includes(:medication, :dosage)
    person_medications = person.person_medications.includes(:medication).ordered

    Result.new(
      person: person,
      schedules: schedules,
      person_medications: person_medications,
      preloaded_takes: {
        schedules: takes_by_schedule(schedules),
        person_medications: takes_by_person_medication(person_medications)
      }
    )
  end

  private

  def takes_by_schedule(schedules)
    MedicationTake
      .where(schedule_id: schedules.map(&:id), taken_at: today_range)
      .includes(:taken_from_location, :taken_from_medication)
      .order(taken_at: :desc)
      .group_by(&:schedule_id)
  end

  def takes_by_person_medication(person_medications)
    MedicationTake
      .where(person_medication_id: person_medications.map(&:id), taken_at: today_range)
      .includes(:taken_from_location, :taken_from_medication)
      .order(taken_at: :desc)
      .group_by(&:person_medication_id)
  end

  def today_range
    Time.current.all_day
  end
end
