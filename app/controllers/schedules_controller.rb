# frozen_string_literal: true

class SchedulesController < ApplicationController
  include TimelineRefreshable
  include PersonViewable
  include TakeMedicationGuardable
  include ScheduleIndexPersonResolvable
  include ScheduleResourceResolvable
  include MedicationWorkflowBackPathable

  before_action :redirect_direct_new_schedule, only: :new
  before_action :set_person, except: %i[index workflow start_workflow frequency_preview]
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

  def frequency_preview
    authorize Schedule.new(person: current_user&.person || Person.new), :create?
    render Components::Schedules::FrequencyPreview.new(
      max_daily_doses: params[:max_daily_doses],
      min_hours_between_doses: params[:min_hours_between_doses],
      dose_cycle: params[:dose_cycle]
    ), layout: false
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

  def redirect_direct_new_schedule
    redirect_to schedules_workflow_path if params[:person_id].blank?
  end

  def set_person
    @person = policy_scope(Person).find(params.expect(:person_id))
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
      frequency: params.expect(:frequency).to_s,
      schedule_type: params.expect(:schedule_type).to_s
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

  def render_schedule_form(schedule:, medications:, title:, **options)
    editing = options.fetch(:editing, false)
    back_path = options[:back_path]
    status = options.fetch(:status, :ok)
    is_modal = request.headers['Turbo-Frame'] == 'modal'

    respond_to do |format|
      format.html do
        if is_modal
          render schedule_modal_component(schedule:, medications:, title:, editing:, back_path:), layout: false, status: status
        else
          render schedule_form_view(schedule:, medications:, editing:), status: status
        end
      end
      format.turbo_stream do
        render turbo_stream: turbo_stream.replace(
          'modal',
          schedule_modal_component(schedule:, medications:, title:, editing:, back_path:)
        ), status: status
      end
    end
  end

  def schedule_modal_component(schedule:, medications:, title:, editing: false, back_path: nil)
    Components::Schedules::Modal.new(
      schedule: schedule,
      person: @person,
      medications: medications,
      title: title,
      editing: editing,
      back_path: back_path
    )
  end

  def schedule_form_view(schedule:, medications:, editing: false)
    if editing
      Components::Schedules::EditView.new(schedule: schedule, person: @person, medications: medications)
    else
      Components::Schedules::NewView.new(schedule: schedule, person: @person, medications: medications)
    end
  end

  def render_schedule_create_success
    respond_to do |format|
      format.html { redirect_to person_path(@person), notice: t('schedules.created') }
      format.turbo_stream do
        flash.now[:notice] = t('schedules.created')
        render turbo_stream: schedule_create_streams
      end
    end
  end

  def schedule_create_streams
    [
      turbo_stream.update('modal', ''),
      turbo_stream.replace(tenant_dom_id(@person), Components::People::PersonCard.new(person: @person.reload)),
      turbo_stream.replace(tenant_dom_target("person_show_#{@person.id}"), person_show_view(@person.reload)),
      turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
    ]
  end

  def render_schedule_create_failure
    @medications = medication_options_query.call
    render_new_schedule_form(status: :unprocessable_content)
  end

  def render_schedule_update_success
    respond_to do |format|
      format.html { redirect_to person_path(@person), notice: t('schedules.updated') }
      format.turbo_stream do
        flash.now[:notice] = t('schedules.updated')
        render turbo_stream: schedule_update_streams
      end
    end
  end

  def schedule_update_streams
    [
      turbo_stream.update('modal', ''),
      turbo_stream.replace(tenant_dom_target("person_show_#{@person.id}"), person_show_view(@person.reload)),
      turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
    ]
  end

  def render_schedule_update_failure
    @medications = medication_options_query.call
    render_edit_schedule_form(status: :unprocessable_content)
  end

  def render_schedule_destroy_success
    respond_to do |format|
      format.html { redirect_back_or_to person_path(@person), notice: t('schedules.deleted') }
      format.turbo_stream do
        flash.now[:notice] = t('schedules.deleted')
        render turbo_stream: [
          turbo_stream.replace(tenant_dom_target("person_show_#{@person.id}"), person_show_view(@person.reload)),
          turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
        ]
      end
    end
  end

  def take_schedule(taken_at)
    result = TakeMedicationService.new.call(
      source: @schedule,
      amount_override: params[:dose_amount],
      taken_from_medication_id: requested_taken_from_medication_id,
      user: current_user,
      taken_at: taken_at
    )
    log_schedule_invalid_take_attempt if result.error == :invalid_amount
    result
  end

  def log_schedule_invalid_take_attempt
    log_invalid_take_attempt(source: 'schedule', amount: nil,
                             metadata: { schedule_id: @schedule.id,
                                         medication_id: @schedule.medication_id })
  end

  def handle_schedule_take_failure(result)
    handle_take_medication_failure(result.error, scope: 'schedules')
  end

  def render_schedule_take_success(take)
    @take = take
    respond_to do |format|
      format.html { redirect_back_or_to person_path(@person), notice: t('schedules.medication_taken') }
      format.turbo_stream do
        flash.now[:notice] = t('schedules.medication_taken')
        streams = build_timeline_streams_for(@schedule.reload, @take)
        streams << turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice]))
        render turbo_stream: streams
      end
    end
  end
end
