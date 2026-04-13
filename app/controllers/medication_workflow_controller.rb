# frozen_string_literal: true

class MedicationWorkflowController < ApplicationController
  def index
    authorize Person, :index?
    people = medication_workflow_people_query.call

    render Components::MedicationWorkflow::PersonSelection.new(people: people, medication_id: params[:medication_id])
  end

  private

  def medication_workflow_people_query
    @medication_workflow_people_query ||= MedicationWorkflowPeopleQuery.new(
      people_scope: policy_scope(Person),
      preload_person: current_user&.person,
      can_add_medication: ->(person) { policy(person).add_medication? }
    )
  end
end
