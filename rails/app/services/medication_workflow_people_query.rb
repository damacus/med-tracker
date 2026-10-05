# frozen_string_literal: true

class MedicationWorkflowPeopleQuery
  attr_reader :people_scope, :preload_person, :can_add_medication

  def initialize(people_scope:, preload_person:, can_add_medication:)
    @people_scope = people_scope
    @preload_person = preload_person
    @can_add_medication = can_add_medication
  end

  def call
    preload_person&.patients&.load

    people_scope.order(:name).select { |person| can_add_medication.call(person) }
  end
end
