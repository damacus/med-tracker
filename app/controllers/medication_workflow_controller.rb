# frozen_string_literal: true

class MedicationWorkflowController < ApplicationController
  def index
    authorize Person, :index?
    # Preload user person and patients to avoid N+1 in policy checks
    current_user.person&.patients&.load

    people = policy_scope(Person).order(:name)
    people = people.select { |person| policy(person).add_medication? }

    render Components::MedicationWorkflow::PersonSelection.new(people: people, medication_id: params[:medication_id])
  end
end
