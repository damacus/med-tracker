# frozen_string_literal: true

module ScheduleWorkflowRenderable
  extend ActiveSupport::Concern

  private

  def render_schedule_workflow
    workflow_options = schedule_workflow_query.options
    @people = workflow_options.people
    @medications = workflow_options.medications
    @selected_person_id = params[:person_id]
    @selected_medication_id = params[:medication_id]
    @schedule_type = params[:schedule_type]
    @frequency = params[:frequency]

    render Components::Schedules::WorkflowView.new(
      people: @people,
      medications: @medications,
      selected_person_id: @selected_person_id,
      selected_medication_id: @selected_medication_id,
      schedule_type: @schedule_type,
      frequency: @frequency
    )
  end

  def selected_schedule_workflow_path
    selection = schedule_workflow_query.selection(
      person_id: params.require(:person_id),
      medication_id: params.require(:medication_id)
    )

    new_person_schedule_path(
      selection.person,
      medication_id: selection.medication.id,
      frequency: params[:frequency].to_s,
      schedule_type: params[:schedule_type].to_s
    )
  end

  def prepare_new_schedule
    @schedule = @person.schedules.build
    @schedule.medication_id = params[:medication_id] if params[:medication_id].present?
    @schedule.frequency = params[:frequency] if params[:frequency].present?
  end

  def render_new_schedule_form(status: :ok)
    render_schedule_form(
      schedule: @schedule,
      medications: @medications,
      title: t('schedules.modal.new_title', person: @person.name),
      back_path: modal_back_path(@person),
      status: status
    )
  end

  def render_edit_schedule_form(status: :ok)
    render_schedule_form(
      schedule: @schedule,
      medications: @medications,
      title: t('schedules.modal.edit_title', person: @person.name),
      editing: true,
      status: status
    )
  end
end
