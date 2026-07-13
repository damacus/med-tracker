# frozen_string_literal: true

class HealthEventsController < ApplicationController
  before_action :set_person
  before_action :set_health_event, only: %i[edit update destroy]

  def index
    authorize HealthEvent.new(person: @person), :index?
    render Views::HealthEvents::Index.new(person: @person, health_events: scoped_health_events)
  end

  def new
    @health_event = @person.health_events.build(event_kind: event_kind_param || :illness)
    authorize @health_event
    render_form
  end

  def edit
    authorize @health_event
    render_form
  end

  def create
    @health_event = @person.health_events.build(health_event_params)
    authorize @health_event

    if save_health_event
      redirect_to person_health_events_path(@person), notice: t('health_events.created')
    else
      render_form(status: :unprocessable_content)
    end
  end

  def update
    authorize @health_event
    @health_event.assign_attributes(health_event_params)

    if save_health_event
      redirect_to person_health_events_path(@person), notice: t('health_events.updated')
    else
      render_form(status: :unprocessable_content)
    end
  end

  def destroy
    authorize @health_event
    @health_event.destroy
    redirect_to person_health_events_path(@person), notice: t('health_events.deleted')
  end

  private

  def set_person
    @person = policy_scope(Person).find(params.expect(:person_id))
    authorize @person, :show?
  end

  def set_health_event
    @health_event = policy_scope(HealthEvent).where(person: @person).find(params.expect(:id))
  end

  def scoped_health_events
    policy_scope(HealthEvent).where(person: @person).includes(:health_event_medications).order(started_on: :desc, id: :desc)
  end

  def render_form(status: :ok)
    render Views::HealthEvents::Form.new(
      person: @person,
      health_event: @health_event,
      medication_options: medication_options,
      selected_medication_ids: selected_medication_ids
    ), status: status
  end

  def save_health_event
    return false unless medication_links_valid?

    ActiveRecord::Base.transaction do
      @health_event.save!
      replace_medication_links
    end
    true
  rescue ActiveRecord::RecordInvalid
    false
  end

  def medication_links_valid?
    return true if invalid_medication_ids.empty?

    @health_event.errors.add(:base, t('health_events.invalid_medication_link'))
    false
  end

  def replace_medication_links
    @health_event.health_event_medications.destroy_all
    return if selected_medications.empty?

    records = selected_medications.map do |medication|
      {
        health_event_id: @health_event.id,
        medication_id: medication.id,
        medication_name: medication.name,
        household_id: @health_event.household_id,
        created_at: Time.current,
        updated_at: Time.current
      }
    end
    HealthEventMedication.insert_all!(records) # rubocop:disable Rails/SkipsModelValidations
    @health_event.refresh_sync_version!
  end

  def health_event_params
    permitted = params.expect(
      health_event: %i[event_kind title started_on ended_on severity notes action_taken medical_help_sought ongoing]
    )
    permitted[:ended_on] = nil if permitted.delete(:ongoing) == '1'
    permitted
  end

  def event_kind_param
    params.permit(:event_kind)[:event_kind].presence_in(HealthEvent.event_kinds.keys)
  end

  def medication_options
    @medication_options ||= policy_scope(Medication).where(id: assigned_medication_ids).order(:name, :id)
  end

  def assigned_medication_ids
    schedule_ids = Schedule.where(person: @person).select(:medication_id)
    person_medication_ids = PersonMedication.where(person: @person).select(:medication_id)
    Medication.where(id: schedule_ids).or(Medication.where(id: person_medication_ids)).select(:id)
  end

  def selected_medication_ids
    @selected_medication_ids ||= Array(params[:medication_ids] || @health_event.medication_ids)
                                 .compact_blank
                                 .map(&:to_i)
  end

  def selected_medications
    @selected_medications ||= medication_options.where(id: selected_medication_ids)
  end

  def invalid_medication_ids
    selected_medication_ids - selected_medications.pluck(:id)
  end
end
