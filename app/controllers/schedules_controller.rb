# frozen_string_literal: true

class SchedulesController < ApplicationController
  include TimelineRefreshable
  include PersonViewable

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

    is_modal = request.headers['Turbo-Frame'] == 'modal'
    modal_back_path = is_modal ? add_medication_person_path(@person) : nil

    respond_to do |format|
      format.html do
        if is_modal
          render Components::Schedules::Modal.new(schedule: @schedule, person: @person, medications: @medications, title: t('schedules.modal.new_title', person: @person.name), back_path: modal_back_path), layout: false
        else
          render Components::Schedules::NewView.new(schedule: @schedule, person: @person, medications: @medications)
        end
      end
      format.turbo_stream do
        render turbo_stream: turbo_stream.replace('modal', Components::Schedules::Modal.new(schedule: @schedule, person: @person, medications: @medications, title: t('schedules.modal.new_title', person: @person.name), back_path: modal_back_path))
      end
    end
  end

  def edit
    authorize @schedule
    @medications = policy_scope(Medication)

    is_modal = request.headers['Turbo-Frame'] == 'modal'

    respond_to do |format|
      format.html do
        if is_modal
          render Components::Schedules::Modal.new(schedule: @schedule, person: @person, medications: @medications, title: t('schedules.modal.edit_title', person: @person.name)), layout: false
        else
          render Components::Schedules::EditView.new(schedule: @schedule, person: @person, medications: @medications)
        end
      end
      format.turbo_stream do
        render turbo_stream: turbo_stream.replace('modal', Components::Schedules::Modal.new(schedule: @schedule, person: @person, medications: @medications, title: t('schedules.modal.edit_title', person: @person.name)))
      end
    end
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
      respond_to do |format|
        format.html { render Components::Schedules::NewView.new(schedule: @schedule, person: @person, medications: @medications), status: :unprocessable_content }
        format.turbo_stream { render turbo_stream: turbo_stream.replace('modal', Components::Schedules::Modal.new(schedule: @schedule, person: @person, medications: @medications, title: t('schedules.modal.new_title', person: @person.name))), status: :unprocessable_content }
      end
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
      respond_to do |format|
        format.html { render Components::Schedules::EditView.new(schedule: @schedule, person: @person, medications: @medications), status: :unprocessable_content }
        format.turbo_stream { render turbo_stream: turbo_stream.replace('modal', Components::Schedules::Modal.new(schedule: @schedule, person: @person, medications: @medications, title: t('schedules.modal.edit_title', person: @person.name))), status: :unprocessable_content }
      end
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

    # SECURITY: Enforce timing restrictions server-side
    # This prevents bypassing UI-disabled buttons via direct API calls
    unless @schedule.can_administer?
      reason = @schedule.administration_blocked_reason
      message = reason == :out_of_stock ? 'Cannot take medication: out of stock' : 'Cannot take medication: timing restrictions not met'
      respond_to do |format|
        format.html do
          redirect_back_or_to person_path(@person),
                              alert: t('schedules.cannot_take_medication', default: message)
        end
        format.turbo_stream do
          flash.now[:alert] = t('schedules.cannot_take_medication', default: message)
          render turbo_stream: turbo_stream.update('flash',
                                                   Components::Layouts::Flash.new(alert: flash[:alert]))
        end
      end
      return
    end

    # Extract the amount from the schedule's dosage if not provided
    amount = params[:amount_ml] || @schedule.dosage.amount

    @take = @schedule.medication_takes.create!(
      taken_at: Time.current,
      amount_ml: amount
    )
    flash.now[:notice] = t('schedules.medication_taken')

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
    @person = Person.find(params[:person_id])
    authorize @person, :show?
  end

  def set_schedule
    @schedule = policy_scope(Schedule).find(params[:id])
  end

  def schedule_params
    params.expect(schedule: %i[medication_id dosage_id frequency start_date end_date notes max_daily_doses min_hours_between_doses dose_cycle])
  end

  def schedule_index_person
    return Person.new if current_user.nil?
    return current_user.person if current_user.person.nil?

    current_user.person.patients.first || current_user.person
  end
end
