# frozen_string_literal: true

class SchedulesController < ApplicationController
  include TimelineRefreshable
  include PersonViewable
  include ScheduleModalRenderable
  include ScheduleMutationRenderable
  include ScheduleTakeRenderable
  include ScheduleWorkflowRenderable
  include TakeMedicationGuardable
  include ScheduleIndexPersonResolvable
  include ScheduleResourceResolvable
  include MedicationWorkflowBackPathable

  before_action :set_person, except: %i[index workflow start_workflow]
  before_action :set_schedule, only: %i[edit update destroy take_medication]

  def index
    authorize Schedule.new(person: schedule_index_person), :index?
    schedules = SchedulesIndexQuery.new(scope: policy_scope(Schedule)).call
    render Components::Schedules::IndexView.new(schedules: schedules)
  end

  def workflow
    authorize Schedule.new(person: current_user&.person || Person.new), :create?
    render_schedule_workflow
  end

  def start_workflow
    authorize Schedule.new(person: current_user&.person || Person.new), :create?
    redirect_to selected_schedule_workflow_path
  end

  def new
    prepare_new_schedule
    authorize @schedule
    @medications = medication_options_query.call
    render_new_schedule_form
  end

  def edit
    authorize @schedule
    @medications = medication_options_query.call
    render_edit_schedule_form
  end

  def create
    @schedule = @person.schedules.build(schedule_params)
    authorize @schedule
    @medications = medication_options_query.call

    if @schedule.save
      render_schedule_create_success
    else
      render_schedule_create_failure
    end
  end

  def update
    authorize @schedule
    if @schedule.update(schedule_params)
      render_schedule_update_success
    else
      render_schedule_update_failure
    end
  end

  def destroy
    authorize @schedule
    @schedule.destroy
    render_schedule_destroy_success
  end

  def take_medication
    authorize @schedule, :take_medication?
    taken_at = medication_taken_at_or_respond(scope: 'schedules')
    return unless taken_at

    result = take_schedule(taken_at)
    return handle_schedule_take_failure(result) unless result.success

    render_schedule_take_success(result.take)
  end

  private

  def set_person
    @person = policy_scope(Person).find(params[:person_id])
    authorize @person, :show?
  end

  def schedule_workflow_query
    @schedule_workflow_query ||= ScheduleWorkflowQuery.new(
      people_scope: policy_scope(Person),
      medications_scope: policy_scope(Medication)
    )
  end

  def medication_options_query
    @medication_options_query ||= MedicationOptionsQuery.new(scope: policy_scope(Medication))
  end
end
