# frozen_string_literal: true

class MedicationWorkflowController < ApplicationController
  def index
    authorize Person, :index?
    people = medication_workflow_people_query.call
    person = requested_person(people)

    if current_account.medication_launcher_variant == 'context_aware' && person && assignment_intent?
      return redirect_to person_assignment_path(person)
    end

    render Components::MedicationWorkflow::PersonSelection.new(
      people: people,
      medication_id: params[:medication_id],
      return_to: url_from(params[:return_to])
    )
  end

  private

  def requested_person(people)
    people.find { |person| person.id.to_s == params[:person_id].to_s }
  end

  def assignment_intent?
    params[:intent].blank? || params[:intent] == 'assign_medication'
  end

  def person_assignment_path(person)
    new_person_medication_assignment_path(
      person,
      source: :workflow,
      medication_id: params[:medication_id],
      return_to: url_from(params[:return_to])
    )
  end

  def medication_workflow_people_query
    @medication_workflow_people_query ||= MedicationWorkflowPeopleQuery.new(
      people_scope: policy_scope(Person),
      preload_person: current_user&.person,
      can_add_medication: ->(person) { policy(person).add_medication? }
    )
  end
end
