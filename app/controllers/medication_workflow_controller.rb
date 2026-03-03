# frozen_string_literal: true

class MedicationWorkflowController < ApplicationController
  def index
    authorize Person, :index?
    people = policy_scope(Person).order(:name)
    render Components::MedicationWorkflow::PersonSelection.new(people: people)
  end
end
