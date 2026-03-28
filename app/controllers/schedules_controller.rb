# frozen_string_literal: true

class SchedulesController < ApplicationController
  include TimelineRefreshable
  include PersonViewable
  include ScheduleModalRenderable
  include TakeMedicationGuardable
  include ScheduleIndexPersonResolvable
  include ScheduleResourceResolvable
  include MedicationWorkflowBackPathable

  before_action :set_person, except: %i[index workflow start_workflow]
  before_action :set_schedule, only: %i[edit update destroy take_medication]

  def index
    authorize Schedule.new(person: schedule_index_person), :index?
    schedules = policy_scope(Schedule).active.includes(:person, :medication, :dosage).order(:start_date, :id)
    render Components::Schedules::IndexView.new(schedules: schedules)
  end

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

  def new
    @schedule = @person.schedules.build
    @schedule.medication_id = params[:medication_id] if params[:medication_id].present?
    @schedule.frequency = params[:frequency] if params[:frequency].present?
    authorize @schedule
    @medications = policy_scope(Medication)
    render_schedule_form(
      schedule: @schedule,
      medications: @medications,
      title: t('schedules.modal.new_title', person: @person.name),
      back_path: modal_back_path(@person)
    )
  end

  def edit
    authorize @schedule
    @medications = policy_scope(Medication)
    render_schedule_form(
      schedule: @schedule,
      medications: @medications,
      title: t('schedules.modal.edit_title', person: @person.name),
      editing: true
    )
  end

  def create
    @schedule = @person.schedules.build(schedule_params)
    authorize @schedule
    @medications = policy_scope(Medication)

    if @schedule.save
      respond_to do |format|
        format.html { redirect_to person_path(@person), notice: t('schedules.created') }
        format.turbo_stream do
          flash.now[:notice] = t('schedules.created')
          render turbo_stream: [
            turbo_stream.update('modal', ''),
            turbo_stream.replace("person_#{@person.id}", Components::People::PersonCard.new(person: @person.reload)),
            turbo_stream.replace("person_show_#{@person.id}", person_show_view(@person.reload)),
            turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
          ]
        end
      end
    else
      render_schedule_form(
        schedule: @schedule,
        medications: @medications,
        title: t('schedules.modal.new_title', person: @person.name),
        status: :unprocessable_content
      )
    end
  end

  def update
    authorize @schedule
    if @schedule.update(schedule_params)
      respond_to do |format|
        format.html { redirect_to person_path(@person), notice: t('schedules.updated') }
        format.turbo_stream do
          flash.now[:notice] = t('schedules.updated')
          render turbo_stream: [
            turbo_stream.update('modal', ''),
            turbo_stream.replace("person_show_#{@person.id}", person_show_view(@person.reload)),
            turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
          ]
        end
      end
    else
      @medications = policy_scope(Medication)
      render_schedule_form(
        schedule: @schedule,
        medications: @medications,
        title: t('schedules.modal.edit_title', person: @person.name),
        editing: true,
        status: :unprocessable_content
      )
    end
  end

  def destroy
    authorize @schedule
    @schedule.destroy
    respond_to do |format|
      format.html { redirect_back_or_to person_path(@person), notice: t('schedules.deleted') }
      format.turbo_stream do
        flash.now[:notice] = t('schedules.deleted')
        render turbo_stream: [
          turbo_stream.replace("person_show_#{@person.id}", person_show_view(@person.reload)),
          turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
        ]
      end
    end
  end

  def take_medication
    authorize @schedule, :take_medication?

    result = TakeMedicationService.new.call(
      source: @schedule,
      amount_override: params[:amount_ml],
      taken_from_medication_id: requested_taken_from_medication_id,
      user: current_user
    )

    if result.error == :invalid_amount
      log_invalid_take_attempt(source: 'schedule', amount: nil,
                               metadata: { schedule_id: @schedule.id,
                                           medication_id: @schedule.medication_id,
                                           dosage_id: @schedule.dosage_id })
    end

    return handle_take_medication_failure(result.error, scope: 'schedules') unless result.success

    @take = result.take
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

  private

  def set_person
    @person = policy_scope(Person).find(params[:person_id])
    authorize @person, :show?
  end
end
