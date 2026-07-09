# frozen_string_literal: true

class MedicationsController < ApplicationController
  include InventoryLocationFilterable
  include MedicationAiSuggestionConfirmation
  include MedicationAdministrationOptions
  include MedicationFormContext
  include MedicationWizardSupport

  before_action :set_medication,
                only: %i[show nhs_guidance administration edit update destroy refill adjust_inventory
                         mark_as_ordered mark_as_received]

  def index
    render_medications_index_for_request
  end

  def show
    authorize @medication
    render Components::Medications::ShowView.new(
      medication: @medication,
      notice: flash[:notice]
    )
  end

  def nhs_guidance
    authorize @medication, :show?

    render Components::Medications::NhsGuidanceFrame.new(
      medication: @medication,
      guidance: NhsWebsiteContent::MedicineGuidanceLookup.new.call(@medication.name)
    ), layout: false
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
      locations: available_locations,
      people: available_people,
      current_user: current_user
    )
  end

  def edit
    authorize @medication
    @return_to = url_from(params[:return_to])
    @medication.dosage_records.build if params[:add_dosage].present?
    render Components::Medications::FormView.new(
      medication: @medication,
      locations: available_locations,
      title: t('medications.form.edit_title'),
      subtitle: t('medications.form.edit_subtitle', name: @medication.name),
      return_to: @return_to
    )
  end

  def create
    @medication = build_medication_from_request
    @medication.location_id ||= primary_location&.id
    authorize @medication

    return if reject_unconfirmed_ai_medication_suggestion?

    result = create_medication_from_request
    @medication = result.medication

    return create_success(notice: create_success_notice(result)) if result.success

    render_create_failure
  end

  def update
    authorize @medication
    @medication.paper_trail_event = 'update'
    if @medication.update(medication_params)
      redirect_update_success
    else
      render_update_failure
    end
  end

  def destroy
    authorize @medication
    destroy_medication
  end

  def mark_as_ordered
    authorize @medication
    update_reorder_status(:ordered, notice: t('medications.ordered'), order_details: order_details_params)
  end

  def mark_as_received
    authorize @medication
    update_reorder_status(:received, notice: t('medications.received'))
  end

  def refill
    authorize @medication, :refill?

    result = restock_medication

    return render_refill_error(result.error) unless result.success?

    render_refill_success
  end

  def adjust_inventory
    authorize @medication, :update?

    result = AdjustMedicationInventoryService.new.call(
      medication: @medication,
      new_quantity: params.dig(:adjustment, :new_quantity),
      reason: params.dig(:adjustment, :reason)
    )

    return render_adjust_error(result.error) unless result.success?

    render_adjust_success
  end

  def scan_restock
    authorize Medication, :index?

    medication = scanned_restock_medication
    return redirect_to medications_path, alert: t('medications.scan_restock_no_match') unless medication

    authorize medication, :refill?
    render_scan_restock_result(restock_scanned_medication(medication))
  end

  def scan_restock_match
    authorize Medication, :index?

    medication = stock_match_resolver.call(barcode: params[:q])
    return render json: { matched: false } unless medication && policy(medication).refill?

    render json: { matched: true, medication: stock_match_payload(medication) }
  end

  def finder
    authorize Medication
    render Components::Medications::FinderView.new
  end

  def search
    authorize Medication, :finder?
    response = medication_finder_search_responder.call(
      query: params[:q],
      form: params[:form],
      permissions: medication_finder_permissions
    )

    render json: response.body, status: response.status
  end

  private

  def restock_scanned_medication(medication)
    RestockMedicationService.new.call(
      medication: medication,
      quantity: params.dig(:inventory_scan, :quantity),
      restock_date: Date.current
    )
  end

  def render_scan_restock_result(result)
    return redirect_to medications_path, alert: result.error unless result.success?

    redirect_to result.medication, notice: t('medications.scan_restocked')
  end

  def set_medication
    @medication = policy_scope(Medication).find(params.expect(:id))
  end

  def render_medications_index_for_request
    @current_category = params[:category]
    base_scope = policy_scope(Medication)
    locations = accessible_inventory_locations(base_scope)
    @current_location_id = resolved_inventory_location_id(locations)

    render_medications_index(
      medication_query: medication_query(base_scope),
      locations: locations
    )
  end

  def medication_query(base_scope)
    MedicationQuery.new(
      scope: base_scope,
      category: @current_category,
      location_id: @current_location_id
    )
  end

  def destroy_medication
    medication_id = @medication.id
    @medication.destroy
    render_destroy_success(medication_id)
  end

  def render_destroy_success(medication_id)
    respond_to do |format|
      format.html { redirect_to medications_url, notice: t('medications.deleted') }
      format.turbo_stream do
        flash.now[:notice] = t('medications.deleted')
        render turbo_stream: destroy_medication_streams(medication_id)
      end
    end
  end

  def destroy_medication_streams(medication_id)
    [
      turbo_stream.remove(tenant_dom_target("medication_#{medication_id}")),
      turbo_stream.remove(tenant_dom_target("medication_show_#{medication_id}")),
      turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
    ]
  end

  def build_medication_from_request
    Medication.new(onboarding_builder.merge_create_attributes(medication_params.to_h.deep_symbolize_keys)).tap do |medication|
      medication.created_by_membership_id = Current.membership&.id
    end
  end

  def redirect_update_success
    redirect_to safe_redirect_path(params[:return_to]) || @medication, notice: t('medications.updated')
  end

  def render_update_failure
    render Components::Medications::FormView.new(
      medication: @medication,
      locations: available_locations,
      title: t('medications.form.edit_title'),
      subtitle: t('medications.form.edit_subtitle', name: @medication.name),
      return_to: url_from(params[:return_to])
    ), status: :unprocessable_content
  end

  def update_reorder_status(status, notice:, order_details: {})
    MedicationReorderStatusService.new.call(medication: @medication, status: status, order_details: order_details)
    respond_to do |format|
      format.html { redirect_back_or_to @medication, notice: notice }
      format.turbo_stream do
        flash.now[:notice] = notice
        render turbo_stream: medication_streams
      end
    end
  end

  def order_details_params
    params.fetch(:order, ActionController::Parameters.new).permit(
      :supplier,
      :quantity,
      :expected_arrival_on
    ).to_h.symbolize_keys
  end

  def restock_medication
    RestockMedicationService.new.call(
      medication: @medication,
      quantity: params.dig(:refill, :quantity),
      restock_date: params.dig(:refill, :restock_date)
    )
  end

  def scanned_restock_medication
    stock_match_resolver.call(barcode: params.dig(:inventory_scan, :barcode))
  end

  def stock_match_resolver
    @stock_match_resolver ||= MedicationStockMatchResolver.new(scope: policy_scope(Medication))
  end

  def stock_match_payload(medication)
    {
      id: medication.id,
      name: medication.display_name,
      full_name: medication.name,
      location: medication.location.name,
      current_supply: MedicationStockQuantityFormatter.format(medication.current_supply),
      path: medication_path(medication),
      refill_path: refill_medication_path(medication)
    }
  end

  def medication_finder_permissions
    policy = policy(Medication)
    {
      can_create: policy.create?,
      can_restock: policy.refill?
    }
  end

  def render_refill_error(message)
    respond_to do |format|
      format.html do
        render Components::Medications::ShowView.new(medication: @medication, notice: message), status: :unprocessable_content
      end
      format.turbo_stream do
        flash.now[:alert] = message
        render turbo_stream: medication_streams, status: :unprocessable_content
      end
    end
  end

  def render_refill_success
    respond_to do |format|
      format.html { redirect_back_or_to @medication, notice: t('medications.refilled') }
      format.turbo_stream do
        flash.now[:notice] = t('medications.refilled')
        render turbo_stream: medication_streams
      end
    end
  end

  def render_adjust_error(message)
    respond_to do |format|
      format.html do
        render Components::Medications::ShowView.new(medication: @medication, notice: message),
               status: :unprocessable_content
      end
      format.turbo_stream do
        flash.now[:alert] = message
        render turbo_stream: medication_streams, status: :unprocessable_content
      end
    end
  end

  def render_adjust_success
    respond_to do |format|
      format.html { redirect_back_or_to @medication, notice: t('medications.inventory_adjusted') }
      format.turbo_stream do
        flash.now[:notice] = t('medications.inventory_adjusted')
        render turbo_stream: medication_streams
      end
    end
  end

  def medication_streams
    medication = @medication.reload
    [
      turbo_stream.replace(tenant_dom_target("medication_show_#{medication.id}"),
                           Components::Medications::ShowView.new(medication: medication)),
      turbo_stream.replace(tenant_dom_id(medication), medication_list_item(medication)),
      turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
    ]
  end

  def medication_list_item(medication)
    Components::Medications::ListItemComponent.new(
      medication: medication,
      inventory_query_params: {},
      can_update: policy(medication).update?,
      can_refill: policy(medication).refill?,
      can_destroy: policy(medication).destroy?
    )
  end

  def create_medication_from_request
    MedicationOnboardingCreateService.new(
      medication: @medication,
      schedule_attributes: onboarding_schedule_params_for_create,
      people_scope: policy_scope(Person),
      medication_scope: policy_scope(Medication),
      plan_authorizer: ->(record) { authorize(record, :create?) }
    ).call
  end

  def create_success_notice(result)
    result.restocked? ? t('medications.refilled') : t('medications.created')
  end

  def onboarding_schedule_params_for_create
    return nil unless params[:wizard] == 'true' && params[:onboarding_schedule].present?

    onboarding_schedule_params.to_h.deep_symbolize_keys
  end
end
