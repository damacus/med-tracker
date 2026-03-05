# frozen_string_literal: true

class SchedulesController < ApplicationController
  include TimelineRefreshable
  include PersonViewable
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
    @schedule.schedule_type = params[:schedule_type] if params[:schedule_type].present?
    @schedule.start_date = Time.zone.today

    authorize @schedule
    render_modal_or_page(Components::Schedules::FormView.new(schedule: @schedule))
  end

  def edit
    authorize @schedule
    render_modal_or_page(Components::Schedules::FormView.new(schedule: @schedule))
  end

  def create
    @schedule = @person.schedules.build(schedule_params)
    authorize @schedule

    if @schedule.save
      redirect_to person_path(@person), notice: 'Schedule was successfully created.'
    else
      render_modal_or_page(Components::Schedules::FormView.new(schedule: @schedule), status: :unprocessable_entity)
    end
  end

  def update
    authorize @schedule
    if @schedule.update(schedule_params)
      redirect_to person_path(@person), notice: 'Schedule was successfully updated.'
    else
      render_modal_or_page(Components::Schedules::FormView.new(schedule: @schedule), status: :unprocessable_entity)
    end
  end

  def destroy
    authorize @schedule
    @schedule.destroy!
    redirect_to person_path(@person), notice: 'Schedule was successfully destroyed.', status: :see_other
  end

  def take_medication
    authorize @schedule

    @take = @schedule.medication_takes.build(
      taken_at: Time.current,
      amount_ml: @schedule.effective_dose_amount
    )

    if @take.save
      respond_to do |format|
        format.html { redirect_to root_path, notice: 'Medication recorded' }
        format.turbo_stream do
          flash.now[:notice] = 'Medication recorded'
          streams = []
          streams << turbo_stream.replace("schedule_#{@schedule.id}",
                                          Components::Dashboard::ScheduleCard.new(schedule: @schedule,
                                                                                  current_user: current_user))
          streams << turbo_stream.prepend('flash-container', Components::Layouts::Flash.new(notice: flash[:notice]))
          render turbo_stream: streams
        end
      end
    else
      respond_to do |format|
        format.html { redirect_to root_path, alert: "Could not record medication: #{@take.errors.full_messages.join(', ')}" }
        format.turbo_stream do
          flash.now[:alert] = "Could not record medication: #{@take.errors.full_messages.join(', ')}"
          streams = []
          streams << turbo_stream.prepend('flash-container', Components::Layouts::Flash.new(alert: flash[:alert]))
          render turbo_stream: streams
        end
      end
    end
  end

  private

  def set_person
    @person = Person.find(params[:person_id])
    authorize @person, :show?
  end

  def set_schedule
    @schedule = @person.schedules.find(params[:id])
  end

  def schedule_params
    params.require(:schedule).permit(:medication_id, :dosage_id, :frequency, :start_date, :end_date, :notes, :active,
                                     :custom_dose_amount, :custom_dose_unit, :schedule_type)
  end

  def render_modal_or_page(component, status: :ok)
    if request.headers['Turbo-Frame'] == 'modal'
      render component, layout: false, status: status
    else
      render component, status: status
    end
  end
end
