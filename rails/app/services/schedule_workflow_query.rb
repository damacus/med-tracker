# frozen_string_literal: true

class ScheduleWorkflowQuery
  Options = Data.define(:people, :medications)
  Selection = Data.define(:person, :medication)

  attr_reader :people_scope, :medications_scope

  def initialize(people_scope:, medications_scope:)
    @people_scope = people_scope
    @medications_scope = medications_scope
  end

  def options
    Options.new(
      people: people_scope.order(:name),
      medications: medications_scope.includes(:location).order(:name)
    )
  end

  def selection(person_id:, medication_id:)
    Selection.new(
      person: people_scope.find(person_id),
      medication: medications_scope.find(medication_id)
    )
  end
end
