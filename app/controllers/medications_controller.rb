# frozen_string_literal: true

class MedicationsController < ApplicationController # rubocop:disable Metrics/ClassLength
  include InventoryLocationFilterable
  include MedicationRefillable

  before_action :set_medication, only: %i[show edit update destroy refill mark_as_ordered mark_as_received]

  def index
    @current_category = params[:category]
    medications = policy_scope(Medication).includes(:location)
    locations = accessible_inventory_locations(medications)
    @current_location_id = resolved_inventory_location_id(locations)

    medications = medications.where(location_id: @current_location_id) if @current_location_id.present?
    categories = medications.where.not(category: [nil, '']).distinct.order(:category).pluck(:category)
    medications = medications.where(category: @current_category) if @current_category.present?

    render Components::Medications::IndexView.new(
      medications: medications,
      current_category: @current_category,
      categories: categories,
      locations: locations,
      current_location_id: @current_location_id
    )
  end

  def show
    authorize @medication
    render Components::Medications::ShowView.new(medication: @medication, notice: flash[:notice])
  end

  def new
    @medication = Medication.new
    @medication.location_id ||= current_primary_location&.id
    authorize @medication

    render wizard_wrapper_class.new(
      medication: @medication,
      locations: available_locations
    )
  end

  def edit
    authorize @medication
    @return_to = params[:return_to]
    render Components::Medications::FormView.new(
      medication: @medication,
      locations: available_locations,
      title: t('medications.form.edit_title'),
      subtitle: t('medications.form.edit_subtitle', name: @medication.name),
      return_to: @return_to
    )
  end

  def create
    @medication = Medication.new(medication_params)
    @medication.location_id ||= current_primary_location&.id
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
      redirect_to params[:return_to].presence || @medication, notice: t('medications.updated')
    else
      render Components::Medications::FormView.new(
        medication: @medication,
        locations: available_locations,
        title: t('medications.form.edit_title'),
        subtitle: t('medications.form.edit_subtitle', name: @medication.name),
        return_to: params[:return_to]
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
      render json: { results: result.results.map(&:to_h) }
    else
      render_medication_search_unavailable
    end
  end

  def dosages
    medication = policy_scope(Medication).find(params[:id])
    authorize medication
    render json: medication.dosages.select(
      :id, :amount, :unit, :description, :frequency,
      :default_for_adults, :default_for_children,
      :default_max_daily_doses, :default_min_hours_between_doses, :default_dose_cycle
    )
  end

  private

  def set_medication
    @medication = policy_scope(Medication).find(params[:id])
  end

  def available_locations
    Location.order(:name)
  end

  def current_primary_location
    return nil unless current_user&.person

    current_user.person.location_memberships.order(:id).first&.location
  end

  def medication_params
    params.expect(
      medication: %i[name
                     category
                     description
                     dosage_amount
                     dosage_unit
                     current_supply
                     reorder_threshold
                     warnings
                     location_id]
    )
  end

  def create_success
    if params[:wizard] == 'true'
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
    case params[:variant]
    when 'modal'     then Components::Medications::Wizard::ModalWrapper
    when 'slideover' then Components::Medications::Wizard::SlideOverWrapper
    else                  Components::Medications::Wizard::FullPageWrapper
    end
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
end
