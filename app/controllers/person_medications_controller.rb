# frozen_string_literal: true

class PersonMedicationsController < ApplicationController
  include TimelineRefreshable
  include PersonViewable
  include TakeMedicationGuardable
  include MedicationWorkflowBackPathable

  before_action :set_person
  before_action :set_person_medication, only: %i[edit update destroy pause resume take_medication reorder]

  def new
    prepare_new_person_medication
    authorize @person_medication
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

  def pause
    authorize @person_medication, :update?
    @person_medication.pause!
    render_person_medication_pause_success
  end

  def resume
    authorize @person_medication, :update?
    @person_medication.resume!
    render_person_medication_resume_success
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
    @person = policy_scope(Person).find(params.expect(:person_id))
    authorize @person, :show?
  end

  def set_person_medication
    @person_medication = @person.person_medications.find(params.expect(:id))
  end

  def prepare_new_person_medication
    @person_medication = @person.person_medications.build
    if medication_options_query.include?(params[:medication_id])
      @person_medication.medication_id = params[:medication_id]
    end
    @medications = medication_options_query.call
  end

  def render_person_medication_form(title:, editing: false, back_path: nil, status: :ok)
    render_modal_or_page(
      modal: -> { person_medication_modal(title: title, editing: editing, back_path: back_path) },
      page: -> { person_medication_form_view(editing: editing) },
      status: status
    )
  end

  def person_medication_modal(title:, editing: false, back_path: nil)
    Components::PersonMedications::Modal.new(
      person_medication: @person_medication,
      person: @person,
      medications: @medications,
      title: title,
      editing: editing,
      back_path: back_path
    )
  end

  def person_medication_form_view(editing: false)
    Components::PersonMedications::FormView.new(
      person_medication: @person_medication,
      person: @person,
      medications: @medications,
      editing: editing
    )
  end

  def save_person_medication?
    return true if explicit_dose_submitted? && @person_medication.save

    add_explicit_dose_errors unless explicit_dose_submitted?
    false
  end

  def render_person_medication_create_failure
    render_person_medication_form(
      title: t('person_medications.modal.new_title', person: @person.name),
      status: :unprocessable_content
    )
  end

  def render_person_medication_update_failure
    render_person_medication_form(
      title: t('person_medications.modal.edit_title', person: @person.name),
      editing: true,
      status: :unprocessable_content
    )
  end

  def render_person_medication_create_success
    respond_to do |format|
      format.html { redirect_to person_path(@person), notice: t('person_medications.created') }
      format.turbo_stream do
        flash.now[:notice] = t('person_medications.created')
        render turbo_stream: person_medication_create_streams
      end
    end
  end

  def person_medication_create_streams
    [
      turbo_stream.update('modal', ''),
      turbo_stream.replace(tenant_dom_id(@person), Components::People::PersonCard.new(person: @person.reload)),
      turbo_stream.replace(tenant_dom_target("person_show_#{@person.id}"), person_show_view(@person.reload)),
      turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
    ]
  end

  def render_person_medication_update_success
    respond_to do |format|
      format.html { redirect_to person_path(@person), notice: t('person_medications.updated') }
      format.turbo_stream do
        flash.now[:notice] = t('person_medications.updated')
        render turbo_stream: person_medication_update_streams
      end
    end
  end

  def person_medication_update_streams
    [
      turbo_stream.update('modal', ''),
      turbo_stream.replace(tenant_dom_target("person_show_#{@person.id}"), person_show_view(@person.reload)),
      turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
    ]
  end

  def render_person_medication_destroy_success
    respond_to do |format|
      format.html { redirect_back_or_to person_path(@person), notice: t('person_medications.deleted') }
      format.turbo_stream do
        flash.now[:notice] = t('person_medications.deleted')
        render turbo_stream: [
          turbo_stream.replace(tenant_dom_target("person_show_#{@person.id}"), person_show_view(@person.reload)),
          turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
        ]
      end
    end
  end

  def render_person_medication_reorder_success
    respond_to do |format|
      format.html { redirect_back_or_to person_path(@person) }
      format.turbo_stream do
        render turbo_stream: turbo_stream.replace(
          tenant_dom_target("person_show_#{@person.id}"),
          person_show_view(@person.reload)
        )
      end
    end
  end

  def render_person_medication_pause_success
    render_person_medication_active_state_success(t('person_medications.paused'))
  end

  def render_person_medication_resume_success
    render_person_medication_active_state_success(t('person_medications.resumed'))
  end

  def render_person_medication_active_state_success(notice)
    respond_to do |format|
      format.html { redirect_to person_path(@person), notice: notice }
      format.turbo_stream do
        flash.now[:notice] = notice
        render turbo_stream: [
          turbo_stream.replace(tenant_dom_target("person_show_#{@person.id}"), person_show_view(@person.reload)),
          turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
        ]
      end
    end
  end

  def take_person_medication(taken_at)
    result = TakeMedicationService.new.call(
      source: @person_medication,
      amount_override: params[:dose_amount],
      taken_from_medication_id: requested_taken_from_medication_id,
      user: current_user,
      taken_at: taken_at
    )
    log_person_medication_invalid_take_attempt if result.error == :invalid_amount
    result
  end

  def log_person_medication_invalid_take_attempt
    log_invalid_take_attempt(source: 'person_medication', amount: nil,
                             metadata: { person_medication_id: @person_medication.id,
                                         medication_id: @person_medication.medication_id })
  end

  def handle_person_medication_take_failure(result)
    handle_take_medication_failure(result.error, scope: 'person_medications')
  end

  def render_person_medication_take_success(take)
    @take = take
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

  def person_medication_params
    params.expect(
      person_medication: %i[
        medication_id
        dose_amount
        dose_unit
        source_dosage_option_id
        administration_kind
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
        administration_kind
        notes
        max_daily_doses
        min_hours_between_doses
        dose_cycle
      ]
    )
  end

  def medication_options_query
    @medication_options_query ||= MedicationOptionsQuery.new(scope: person_medication_inventory_scope)
  end

  def person_medication_inventory_scope
    return policy_scope(Medication) unless Current.household

    Medication.where(household: Current.household)
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
