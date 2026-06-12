# frozen_string_literal: true

class MedicationTimelineQuery
  Result = Data.define(:schedules, :person_medications)

  attr_reader :medication, :excluding, :schedules_scope, :person_medications_scope

  def initialize(medication:, excluding: nil, schedules_scope: Schedule.all,
                 person_medications_scope: PersonMedication.all)
    @medication = medication
    @excluding = excluding
    @schedules_scope = schedules_scope
    @person_medications_scope = person_medications_scope
  end

  def call
    Result.new(
      schedules: schedules,
      person_medications: person_medications
    )
  end

  private

  def schedules
    relation = schedules_scope.active.where(medication: medication).includes(:person, :medication)
    excluding.is_a?(Schedule) ? relation.where.not(id: excluding.id) : relation
  end

  def person_medications
    relation = person_medications_scope.where(medication: medication).includes(:person, :medication)
    excluding.is_a?(PersonMedication) ? relation.where.not(id: excluding.id) : relation
  end
end
