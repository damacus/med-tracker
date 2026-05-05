# frozen_string_literal: true

class PersonMedicationsController < ApplicationController
  include TimelineRefreshable
  include PersonViewable
  include PersonMedicationFormRenderable
  include PersonMedicationResponseRenderable
  include PersonMedicationTakeRenderable
  include TakeMedicationGuardable
  include MedicationWorkflowBackPathable

  before_action :set_person
  before_action :set_person_medication, only: %i[edit update destroy take_medication reorder]

  def new
    authorize PersonMedication
    prepare_new_person_medication
    render_person_medication_form(
      title: t('person_medications.modal.new_title', person: @person.name),
      back_path: modal_back_path(@person)
    )
  end

  def edit
    authorize @person_medication
    @medications = medication_options_query.call
    render_person_medication_form(
      title: t('person_medications.modal.edit_title', person: @person.name),
      editing: true
    )
  end

  def create
    @person_medication = @person.person_medications.build(person_medication_params)
    authorize @person_medication
    @medications = medication_options_query.call

    if save_person_medication?
      render_person_medication_create_success
    else
      render_person_medication_create_failure
    end
  end

  def update
    authorize @person_medication
    @medications = medication_options_query.call

    if @person_medication.update(person_medication_update_params)
      render_person_medication_update_success
    else
      render_person_medication_update_failure
    end
  end

  def destroy
    authorize @person_medication
    @person_medication.destroy
    render_person_medication_destroy_success
  end

  def reorder
    authorize @person_medication, :update?
    PersonMedicationReorderService.new.call(person_medication: @person_medication, direction: params[:direction])
    render_person_medication_reorder_success
  end

  def take_medication
    authorize @person_medication, :take_medication?
    taken_at = medication_taken_at_or_respond(scope: 'person_medications')
    return unless taken_at

    result = take_person_medication(taken_at)
    return handle_person_medication_take_failure(result) unless result.success

    render_person_medication_take_success(result.take)
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
    params.expect(
      person_medication: %i[
        medication_id
        dose_amount
        dose_unit
        source_dosage_option_id
        notes
        max_daily_doses
        min_hours_between_doses
        dose_cycle
      ]
    )
  end

  def person_medication_update_params
    params.expect(
      person_medication: %i[
        dose_amount
        dose_unit
        source_dosage_option_id
        notes
        max_daily_doses
        min_hours_between_doses
        dose_cycle
      ]
    )
  end

  def medication_options_query
    @medication_options_query ||= MedicationOptionsQuery.new(scope: policy_scope(Medication))
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
