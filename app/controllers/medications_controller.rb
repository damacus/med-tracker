# frozen_string_literal: true

class MedicationsController < ApplicationController
  include InventoryLocationFilterable
  include MedicationAdministrationOptions
  include MedicationFormContext
  include MedicationRefillable
  include MedicationWizardSupport

  before_action :set_medication, only: %i[show administration edit update destroy refill mark_as_ordered mark_as_received]

  def index
    @current_category = params[:category]
    base_scope = policy_scope(Medication)
    locations = accessible_inventory_locations(base_scope)
    @current_location_id = resolved_inventory_location_id(locations)

    medication_query = MedicationQuery.new(
      scope: base_scope,
      category: @current_category,
      location_id: @current_location_id
    )

    render Components::Medications::IndexView.new(
      medications: medication_query.call,
      current_category: @current_category,
      categories: medication_query.categories,
      locations: locations,
      current_location_id: @current_location_id,
      wizard_variant: current_user.wizard_variant
    )
  end

  def show
    authorize @medication
    render Components::Medications::ShowView.new(
      medication: @medication,
      notice: flash[:notice],
      nhs_guidance: NhsWebsiteContent::MedicineGuidanceLookup.new.call(@medication.name)
    )
  end

  def administration
    authorize @medication, :show?

    render Components::Medications::AdministrationModal.new(
      medication: @medication,
      schedules: administration_schedules,
      person_medications: administration_person_medications,
      current_user: current_user
    ), layout: false
  end

  def new
    @medication = Medication.new
    @medication.location_id ||= primary_location&.id
    authorize @medication
    onboarding_builder.build_new(medication: @medication, params: params)

    render wizard_wrapper_class.new(
      medication: @medication,
      locations: available_locations
    )
  end

  def edit
    authorize @medication
    @return_to = url_from(params[:return_to])
    render Components::Medications::FormView.new(
      medication: @medication,
      locations: available_locations,
      title: t('medications.form.edit_title'),
      subtitle: t('medications.form.edit_subtitle', name: @medication.name),
      return_to: @return_to
    )
  end

  def create
    @medication = Medication.new(onboarding_builder.merge_create_attributes(medication_params.to_h.deep_symbolize_keys))
    @medication.location_id ||= primary_location&.id
    authorize @medication

    if @medication.save
      create_success
    elsif params[:wizard] == 'true'
      render wizard_wrapper_class.new(
        medication: @medication,
        locations: available_locations
      ), status: :unprocessable_content
    else
      render Components::Medications::FormView.new(
        medication: @medication,
        locations: available_locations,
        title: t('medications.form.new_title'),
        subtitle: t('medications.form.new_subtitle')
      ), status: :unprocessable_content
    end
  end

  def update
    authorize @medication
    if @medication.update(medication_params)
      redirect_to safe_redirect_path(params[:return_to]) || @medication, notice: t('medications.updated')
    else
      render Components::Medications::FormView.new(
        medication: @medication,
        locations: available_locations,
        title: t('medications.form.edit_title'),
        subtitle: t('medications.form.edit_subtitle', name: @medication.name),
        return_to: url_from(params[:return_to])
      ), status: :unprocessable_content
    end
  end

  def destroy
    authorize @medication
    @medication.destroy
    redirect_to medications_url, notice: t('medications.deleted')
  end

  def mark_as_ordered
    authorize @medication
    @medication.update!(reorder_status: :ordered, ordered_at: Time.current)
    respond_to do |format|
      format.html { redirect_back_or_to @medication, notice: t('medications.ordered') }
      format.turbo_stream do
        flash.now[:notice] = t('medications.ordered')
        render turbo_stream: medication_streams
      end
    end
  end

  def mark_as_received
    authorize @medication
    @medication.update!(reorder_status: :received, reordered_at: Time.current)
    respond_to do |format|
      format.html { redirect_back_or_to @medication, notice: t('medications.received') }
      format.turbo_stream do
        flash.now[:notice] = t('medications.received')
        render turbo_stream: medication_streams
      end
    end
  end

  def refill
    authorize @medication, :update?

    quantity = refill_quantity
    restock_date = parse_restock_date

    if quantity <= 0
      render_refill_error('Quantity must be greater than 0')
      return
    end

    unless restock_date
      render_refill_error('Restock date is invalid')
      return
    end

    @medication.paper_trail_event = "restock (qty: #{quantity}, date: #{restock_date.iso8601})"
    @medication.restock!(quantity: quantity)

    respond_to do |format|
      format.html { redirect_back_or_to @medication, notice: t('medications.refilled') }
      format.turbo_stream do
        flash.now[:notice] = t('medications.refilled')
        render turbo_stream: medication_streams
      end
    end
  rescue ActiveRecord::RecordInvalid => e
    render_refill_error(e.record.errors.full_messages.to_sentence)
  end

  def finder
    authorize Medication
    render Components::Medications::FinderView.new
  end

  def search
    authorize Medication, :finder?
    response = medication_finder_search_responder.call(query: params[:q])

    render json: response.body, status: response.status
  end

  private

  def set_medication
    @medication = policy_scope(Medication).find(params[:id])
  end
end
