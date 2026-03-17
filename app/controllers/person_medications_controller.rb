# frozen_string_literal: true

class PersonMedicationsController < ApplicationController
  include TimelineRefreshable
  include PersonViewable
  include TakeMedicationGuardable
  include MedicationWorkflowBackPathable

  before_action :set_person
  before_action :set_person_medication, only: %i[edit update destroy take_medication reorder]

  def new
    authorize PersonMedication
    @person_medication = @person.person_medications.build
    @person_medication.medication_id = params[:medication_id] if medication_in_scope?(params[:medication_id])
    @medications = available_medications

    is_modal = request.headers['Turbo-Frame'] == 'modal'
    back_path = modal_back_path(@person)

    respond_to do |format|
      format.html do
        if is_modal
          render Components::PersonMedications::Modal.new(person_medication: @person_medication, person: @person, medications: @medications, title: t('person_medications.modal.new_title', person: @person.name), back_path: back_path), layout: false
        else
          render Components::PersonMedications::FormView.new(person_medication: @person_medication, person: @person, medications: @medications)
        end
      end
      format.turbo_stream do
        render turbo_stream: turbo_stream.replace('modal', Components::PersonMedications::Modal.new(person_medication: @person_medication, person: @person, medications: @medications, title: t('person_medications.modal.new_title', person: @person.name), back_path: back_path))
      end
    end
  end

  def edit
    authorize @person_medication
    @medications = available_medications

    is_modal = request.headers['Turbo-Frame'] == 'modal'

    respond_to do |format|
      format.html do
        if is_modal
          render Components::PersonMedications::Modal.new(person_medication: @person_medication, person: @person, medications: @medications, title: t('person_medications.modal.edit_title', person: @person.name), editing: true), layout: false
        else
          render Components::PersonMedications::FormView.new(person_medication: @person_medication, person: @person, medications: @medications, editing: true)
        end
      end
      format.turbo_stream do
        render turbo_stream: turbo_stream.replace('modal', Components::PersonMedications::Modal.new(person_medication: @person_medication, person: @person, medications: @medications, title: t('person_medications.modal.edit_title', person: @person.name), editing: true))
      end
    end
  end

  def create
    @person_medication = @person.person_medications.build(person_medication_params)
    authorize @person_medication
    @medications = available_medications

    if explicit_dose_submitted? && @person_medication.save
      respond_to do |format|
        format.html { redirect_to person_path(@person), notice: t('person_medications.created') }
        format.turbo_stream do
          flash.now[:notice] = t('person_medications.created')
          render turbo_stream: [
            turbo_stream.update('modal', ''),
            turbo_stream.replace("person_#{@person.id}", Components::People::PersonCard.new(person: @person.reload)),
            turbo_stream.replace("person_show_#{@person.id}", person_show_view(@person.reload)),
            turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
          ]
        end
      end
    else
      add_explicit_dose_errors unless explicit_dose_submitted?

      respond_to do |format|
        format.html { render Components::PersonMedications::FormView.new(person_medication: @person_medication, person: @person, medications: @medications), status: :unprocessable_content }
        format.turbo_stream { render turbo_stream: turbo_stream.replace('modal', Components::PersonMedications::Modal.new(person_medication: @person_medication, person: @person, medications: @medications, title: t('person_medications.modal.new_title', person: @person.name))), status: :unprocessable_content }
      end
    end
  end

  def update
    authorize @person_medication
    @medications = available_medications

    if @person_medication.update(person_medication_update_params)
      respond_to do |format|
        format.html { redirect_to person_path(@person), notice: t('person_medications.updated') }
        format.turbo_stream do
          flash.now[:notice] = t('person_medications.updated')
          render turbo_stream: [
            turbo_stream.update('modal', ''),
            turbo_stream.replace("person_show_#{@person.id}", person_show_view(@person.reload)),
            turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
          ]
        end
      end
    else
      respond_to do |format|
        format.html { render Components::PersonMedications::FormView.new(person_medication: @person_medication, person: @person, medications: @medications, editing: true), status: :unprocessable_content }
        format.turbo_stream { render turbo_stream: turbo_stream.replace('modal', Components::PersonMedications::Modal.new(person_medication: @person_medication, person: @person, medications: @medications, title: t('person_medications.modal.edit_title', person: @person.name), editing: true)), status: :unprocessable_content }
      end
    end
  end

  def destroy
    authorize @person_medication
    @person_medication.destroy
    respond_to do |format|
      format.html { redirect_back_or_to person_path(@person), notice: t('person_medications.deleted') }
      format.turbo_stream do
        flash.now[:notice] = t('person_medications.deleted')
        render turbo_stream: [
          turbo_stream.replace("person_show_#{@person.id}", person_show_view(@person.reload)),
          turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
        ]
      end
    end
  end

  def reorder
    authorize @person_medication, :update?
    @person_medication.reorder(params[:direction])
    respond_to do |format|
      format.html { redirect_back_or_to person_path(@person) }
      format.turbo_stream do
        render turbo_stream: turbo_stream.replace("person_show_#{@person.id}", person_show_view(@person.reload))
      end
    end
  end

  def take_medication
    authorize @person_medication, :take_medication?

    reason = take_medication_blocked_reason(@person_medication)
    if reason
      message = reason == :out_of_stock ? 'Cannot take medication: out of stock' : 'Cannot take medication: timing restrictions not met'
      respond_to do |format|
        format.html do
          redirect_back_or_to person_path(@person),
                              alert: t('person_medications.cannot_take_medication', default: message)
        end
        format.turbo_stream do
          flash.now[:alert] = t('person_medications.cannot_take_medication', default: message)
          render turbo_stream: turbo_stream.update('flash',
                                                   Components::Layouts::Flash.new(alert: flash[:alert]))
        end
      end
      return
    end

    amount = normalized_take_amount(params[:amount_ml].presence || @person_medication.dose_amount)
    if invalid_take_amount?(amount)
      log_invalid_take_attempt(
        source: 'person_medication',
        amount: amount,
        metadata: {
          person_medication_id: @person_medication.id,
          medication_id: @person_medication.medication_id
        }
      )
      respond_take_medication_invalid_dose(scope: 'person_medications')
      return
    end

    stock_source_error, taken_from_medication = resolve_taken_from_medication(@person_medication)
    if stock_source_error
      respond_take_medication_stock_source_error(scope: 'person_medications', error: stock_source_error)
      return
    end

    @take = @person_medication.medication_takes.create(
      taken_at: Time.current,
      amount_ml: amount,
      taken_from_medication: taken_from_medication,
      taken_from_location: taken_from_medication.location
    )
    unless @take.persisted?
      respond_take_medication_invalid_dose(scope: 'person_medications')
      return
    end

    flash.now[:notice] = t('person_medications.medication_taken')

    respond_to do |format|
      format.html { redirect_back_or_to person_path(@person), notice: t('person_medications.medication_taken') }

      format.turbo_stream do
        flash.now[:notice] = t('person_medications.medication_taken')
        streams = build_timeline_streams_for(@person_medication.reload, @take)
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

  def set_person_medication
    @person_medication = @person.person_medications.find(params[:id])
  end

  def person_medication_params
    params.expect(person_medication: %i[medication_id dose_amount dose_unit notes max_daily_doses min_hours_between_doses dose_cycle])
  end

  def person_medication_update_params
    params.expect(person_medication: %i[dose_amount dose_unit notes max_daily_doses min_hours_between_doses dose_cycle])
  end

  def available_medications
    policy_scope(Medication).order(:name)
  end

  def medication_in_scope?(medication_id)
    available_medications.exists?(id: medication_id)
  end

  def explicit_dose_submitted?
    params.dig(:person_medication, :dose_amount).present? &&
      params.dig(:person_medication, :dose_unit).present?
  end

  def add_explicit_dose_errors
    @person_medication.errors.add(:dose_amount, :blank) if @person_medication.errors[:dose_amount].blank?
    @person_medication.errors.add(:dose_unit, :blank) if @person_medication.errors[:dose_unit].blank?
  end
end
