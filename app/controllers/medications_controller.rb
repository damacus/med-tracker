# frozen_string_literal: true

class MedicationsController < ApplicationController
  before_action :set_medication, only: %i[show edit update destroy refill]

  def index
    medications = policy_scope(Medication)
    render Components::Medications::IndexView.new(medications: medications)
  end

  def show
    authorize @medication
    render Components::Medications::ShowView.new(medication: @medication, notice: flash[:notice])
  end

  def new
    @medication = Medication.new
    authorize @medication
    render Components::Medications::FormView.new(
      medication: @medication,
      locations: available_locations,
      title: t('medications.form.new_title'),
      subtitle: t('medications.form.new_subtitle')
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
    authorize @medication

    if @medication.save
      redirect_to @medication, notice: t('medications.created')
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
    @medication = Medication.find(params[:id])
    authorize @medication
    @medication.update!(reorder_status: :ordered, ordered_at: Time.current)
    redirect_back_or_to @medication, notice: t('medications.ordered')
  end

  def mark_as_received
    @medication = Medication.find(params[:id])
    authorize @medication
    @medication.update!(reorder_status: :received, reordered_at: Time.current)
    redirect_back_or_to @medication, notice: t('medications.received')
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

    redirect_back_or_to @medication, notice: t('medications.refilled')
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
    result = NhsDmd::Search.new.call(query)

    if result.success?
      render json: { results: result.results.map(&:to_h) }
    else
      render json: { results: [], error: result.error }
    end
  end

  def dosages
    medication = policy_scope(Medication).find(params[:id])
    authorize medication
    render json: medication.dosages.select(:id, :amount, :unit, :description)
  end

  private

  def set_medication
    @medication = policy_scope(Medication).find(params[:id])
  end

  def available_locations
    Location.order(:name)
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

  def refill_quantity
    params.dig(:refill, :quantity).to_i
  end

  def parse_restock_date
    Date.parse(params.dig(:refill, :restock_date).to_s)
  rescue ArgumentError
    nil
  end

  def render_refill_error(message)
    render Components::Medications::ShowView.new(medication: @medication, notice: message), status: :unprocessable_content
  end
end
