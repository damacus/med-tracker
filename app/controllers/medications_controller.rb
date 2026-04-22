# frozen_string_literal: true

class MedicationsController < ApplicationController # rubocop:disable Metrics/ClassLength
  include InventoryLocationFilterable
  include MedicationRefillable

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
    @medication.assign_attributes(finder_prefill_attributes)
    apply_onboarding_prefill(@medication)

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
    @medication = Medication.new(apply_onboarding_prefill_to_attributes(medication_params.to_h.deep_symbolize_keys))
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
    query = params[:q].to_s.strip
    return render json: { results: [] } if query.blank?

    result = search_results_for(query)
    return render_medication_search_unavailable unless result

    if result.success?
      render json: {
        results: result.results.map(&:to_h),
        query: result.resolved_query.presence || query,
        barcode: result.barcode
      }
    else
      render_medication_search_unavailable
    end
  end

  private

  def set_medication
    @medication = policy_scope(Medication).find(params[:id])
  end

  def available_locations
    LocationsQuery.new(scope: policy_scope(Location)).options
  end

  def primary_location
    PrimaryLocationQuery.new(person: current_user&.person).call
  end

  def medication_params
    params.expect(
      medication: [
        :name,
        :barcode,
        :dmd_code,
        :dmd_system,
        :dmd_concept_class,
        :category,
        :description,
        :dosage_amount,
        :dosage_unit,
        :current_supply,
        :reorder_threshold,
        :warnings,
        :location_id,
        {
          dosage_records_attributes: %i[
            id
            amount
            unit
            frequency
            description
            default_for_adults
            default_for_children
            default_max_daily_doses
            default_min_hours_between_doses
            default_dose_cycle
            current_supply
            reorder_threshold
            _destroy
          ]
        }
      ]
    )
  end

  def create_success
    if params[:wizard] == 'true'
      seed_initial_dosage
      respond_to do |format|
        format.turbo_stream do
          render turbo_stream: turbo_stream.replace(
            'wizard-content',
            Components::Medications::Wizard::StepDosages.new(medication: @medication)
          )
        end
        format.html { redirect_to @medication, notice: t('medications.created') }
      end
    else
      redirect_to @medication, notice: t('medications.created')
    end
  end

  def wizard_wrapper_class
    case current_user.wizard_variant
    when 'modal'     then Components::Medications::Wizard::ModalWrapper
    when 'slideover' then Components::Medications::Wizard::SlideOverWrapper
    else                  Components::Medications::Wizard::FullPageWrapper
    end
  end

  def seed_initial_dosage
    return if @medication.dosage_records.exists?
    return unless @medication.dosage_amount.present? && @medication.dosage_unit.present?

    @medication.dosage_records.create!(
      amount: @medication.dosage_amount,
      unit: @medication.dosage_unit,
      frequency: 'As directed',
      default_for_adults: true,
      default_max_daily_doses: 1,
      default_min_hours_between_doses: 24,
      default_dose_cycle: :daily
    )
  end

  def search_results_for(query)
    NhsDmd::Search.new.call(query)
  rescue StandardError => e
    Rails.logger.error("Medication finder search failed: #{e.class}: #{e.message}")
    nil
  end

  def render_medication_search_unavailable
    render json: { results: [], error: 'Medication search is temporarily unavailable.' }, status: :service_unavailable
  end

  def finder_prefill_attributes
    attrs = {}
    attrs[:name] = params[:name].presence if params[:name].present?

    barcode = params[:barcode].presence
    attrs[:barcode] = barcode if NhsDmd::BarcodeLookup.barcode_query?(barcode)
    attrs[:dmd_code] = params[:dmd_code].presence if params[:dmd_code].present?
    attrs[:dmd_system] = params[:dmd_system].presence if params[:dmd_code].present?
    attrs[:dmd_concept_class] = params[:dmd_concept_class].presence if params[:dmd_code].present?
    attrs
  end

  def apply_onboarding_prefill(medication)
    defaults = onboarding_prefill_for(
      barcode: medication.barcode,
      code: medication.dmd_code,
      name: medication.name
    )

    defaults.medication_attributes.each do |key, value|
      medication.public_send("#{key}=", value)
    end
    build_onboarding_dosage_records!(medication, defaults.dosage_records_attributes)
  end

  def apply_onboarding_prefill_to_attributes(attrs)
    defaults = onboarding_prefill_for(
      barcode: attrs[:barcode],
      code: attrs[:dmd_code],
      name: attrs[:name]
    )
    explicit_inventory_override = explicit_inventory_override?(attrs)

    merge_onboarding_medication_defaults!(attrs, defaults.medication_attributes)
    merge_onboarding_dosage_defaults!(
      attrs,
      defaults.dosage_records_attributes,
      explicit_inventory_override: explicit_inventory_override
    )

    attrs
  end

  def onboarding_prefill_for(barcode:, code:, name:)
    MedicationOnboardingPrefill.new.call(barcode: barcode, code: code, name: name)
  end

  def dosage_records_blank?(dosage_records_attributes)
    return true if dosage_records_attributes.blank?

    dosage_records_attributes.values.all? do |attributes|
      attributes.except(:id, :_destroy, :default_dose_cycle).values.all?(&:blank?)
    end
  end

  def merge_onboarding_medication_defaults!(attrs, defaults)
    defaults.each do |key, value|
      assign_onboarding_attribute!(attrs, key, value)
    end
  end

  def merge_onboarding_dosage_defaults!(attrs, dosage_defaults, explicit_inventory_override:)
    return unless dosage_records_blank?(attrs[:dosage_records_attributes]) && dosage_defaults.any?

    attrs[:dosage_records_attributes] = serialized_onboarding_dosages(
      dosage_defaults_for_merge(dosage_defaults, explicit_inventory_override:)
    )
  end

  def dosage_defaults_for_merge(dosage_defaults, explicit_inventory_override:)
    return dosage_defaults unless explicit_inventory_override

    dosage_defaults.map { |dosage| dosage.except(:current_supply, :reorder_threshold) }
  end

  def serialized_onboarding_dosages(dosage_defaults)
    dosage_defaults.each_with_index.to_h do |dosage, index|
      [index.to_s, dosage]
    end
  end

  def explicit_inventory_override?(attrs)
    attrs[:current_supply].present? || attrs[:reorder_threshold].present?
  end

  def build_onboarding_dosage_records!(medication, dosage_defaults)
    return if medication.dosage_records.any? || dosage_defaults.blank?

    dosage_defaults.each do |attributes|
      medication.dosage_records.build(attributes)
    end
  end

  def assign_onboarding_attribute!(target, key, value)
    if target.respond_to?(:[])
      target[key] = value if target[key].blank?
      return
    end

    target.public_send("#{key}=", value) if target.public_send(key).blank?
  end

  def administration_schedules
    policy_scope(Schedule)
      .includes(:person, :medication)
      .where(medication: @medication)
      .active
      .select { |schedule| policy(schedule).take_medication? }
      .sort_by { |schedule| [schedule.person.name, schedule.id] }
  end

  def administration_person_medications
    policy_scope(PersonMedication)
      .includes(:person, :medication)
      .where(medication: @medication)
      .select { |person_medication| policy(person_medication).take_medication? }
      .sort_by { |person_medication| [person_medication.person.name, person_medication.id] }
  end
end
