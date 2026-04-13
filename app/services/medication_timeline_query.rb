# frozen_string_literal: true

class MedicationTimelineQuery
  Result = Data.define(:schedules, :person_medications)

  attr_reader :medication, :excluding

  def initialize(medication:, excluding: nil)
    @medication = medication
    @excluding = excluding
  end

  def call
    Result.new(
      schedules: schedules,
      person_medications: person_medications
    )
  end

  private

  def schedules
    relation = Schedule.active.where(medication: medication).includes(:person, :medication, :dosage)
    excluding.is_a?(Schedule) ? relation.where.not(id: excluding.id) : relation
  end

  def person_medications
    relation = PersonMedication.where(medication: medication).includes(:person, :medication)
    excluding.is_a?(PersonMedication) ? relation.where.not(id: excluding.id) : relation
  end
end
