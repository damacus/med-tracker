# frozen_string_literal: true

module ScheduleWorkflowActions
  def workflow
    authorize Schedule.new(person: current_user&.person || Person.new), :create?
    @people = policy_scope(Person).order(:name)
    @medications = policy_scope(Medication).includes(:location).order(:name)
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

  def start_workflow
    authorize Schedule.new(person: current_user&.person || Person.new), :create?
    people = policy_scope(Person)
    medications = policy_scope(Medication)

    person = people.find(params.require(:person_id))
    medication = medications.find(params.require(:medication_id))
    frequency = params[:frequency].to_s
    schedule_type = params[:schedule_type].to_s

    redirect_to new_person_schedule_path(
      person,
      medication_id: medication.id,
      frequency: frequency,
      schedule_type: schedule_type
    )
  end
end
