# frozen_string_literal: true

class MedicationAssignmentsController < ApplicationController
  include PersonViewable

  before_action :set_person
  before_action :authorize_assignment

  def new
    @assignment = MedicationAssignment.new(medication_id: preselected_medication_id)
    @medications = medication_options_query.call

    render_assignment_form
  end

  def create
    @assignment = MedicationAssignment.new(assignment_params)
    ensure_medication_access
    @medications = medication_options_query.call

    result = MedicationAssignmentCreator
      .new(
        person: @person,
        medication_scope: policy_scope(Medication),
        assignment: @assignment
      )
      .call

    if result.success
      respond_to do |format|
        format.html { redirect_to(person_path(@person), notice: t("schedules.created")) }
        format.turbo_stream do
          flash.now[:notice] = t("schedules.created")
          render(
            turbo_stream: [
              turbo_stream.update("modal", ""),
              turbo_stream.replace("person_#{@person.id}", Components::People::PersonCard.new(person: @person.reload)),
              turbo_stream.replace("person_show_#{@person.id}", person_show_view(@person.reload)),
              turbo_stream.update("flash", Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
            ]
          )
        end
      end
    else
      @assignment = result.assignment
      render_assignment_form(status: :unprocessable_content)
    end
  end

  private

  def set_person
    @person = policy_scope(Person).find(params[:person_id])
    authorize(@person, :show?)
  end

  def authorize_assignment
    return if policy(Schedule.new(person: @person)).create?
    return if policy(PersonMedication.new(person: @person)).create?

    raise Pundit::NotAuthorizedError
  end

  def ensure_medication_access
    return if @assignment.medication_id.blank?
    return if medication_options_query.include?(@assignment.medication_id)

    raise Pundit::NotAuthorizedError
  end

  def assignment_params
    params.expect(
      medication_assignment: %i[
        medication_id
        source_dosage_option_id
        dose_amount
        dose_unit
      ]
    )
  end

  def preselected_medication_id
    return params[:medication_id] if medication_options_query.include?(params[:medication_id])

    nil
  end

  def medication_options_query
    @medication_options_query ||= MedicationOptionsQuery.new(scope: policy_scope(Medication))
  end

  def render_assignment_form(status: :ok)
    back_path = params[:source] == "workflow" ? add_medication_path(medication_id: params[:medication_id]) : nil
    is_modal = request.headers["Turbo-Frame"] == "modal"

    respond_to do |format|
      format.html do
        if is_modal
          render(
            Components::MedicationAssignments::Modal.new(
              assignment: @assignment,
              person: @person,
              medications: @medications,
              back_path: back_path
            ),
            layout: false,
            status: status
          )
        else
          render(
            Components::MedicationAssignments::FormView.new(
              assignment: @assignment,
              person: @person,
              medications: @medications
            ),
            status: status
          )
        end
      end

      format.turbo_stream do
        render(
          turbo_stream: turbo_stream.replace(
            "modal",
            Components::MedicationAssignments::Modal.new(
              assignment: @assignment,
              person: @person,
              medications: @medications,
              back_path: back_path
            )
          ),
          status: status
        )
      end
    end
  end
end
