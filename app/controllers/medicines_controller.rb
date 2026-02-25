# frozen_string_literal: true

class MedicinesController < ApplicationController
  before_action :set_medicine, only: %i[show edit update destroy refill]

  def index
    medicines = policy_scope(Medicine)
    render Components::Medicines::IndexView.new(medicines: medicines)
  end

  def show
    authorize @medicine
    render Components::Medicines::ShowView.new(medicine: @medicine, notice: flash[:notice])
  end

  def new
    @medicine = Medicine.new
    authorize @medicine
    render Components::Medicines::FormView.new(
      medicine: @medicine,
      locations: available_locations,
      title: t('medicines.form.new_title'),
      subtitle: t('medicines.form.new_subtitle')
    )
  end

  def edit
    authorize @medicine
    render Components::Medicines::FormView.new(
      medicine: @medicine,
      locations: available_locations,
      title: t('medicines.form.edit_title'),
      subtitle: t('medicines.form.edit_subtitle', name: @medicine.name)
    )
  end

  def create
    @medicine = Medicine.new(medicine_params)
    authorize @medicine

    if @medicine.save
      redirect_to @medicine, notice: t('medicines.created')
    else
      render Components::Medicines::FormView.new(
        medicine: @medicine,
        locations: available_locations,
        title: t('medicines.form.new_title'),
        subtitle: t('medicines.form.new_subtitle')
      ), status: :unprocessable_content
    end
  end

  def update
    authorize @medicine
    if @medicine.update(medicine_params)
      redirect_back_or_to @medicine, notice: t('medicines.updated')
    else
      render Components::Medicines::FormView.new(
        medicine: @medicine,
        locations: available_locations,
        title: t('medicines.form.edit_title'),
        subtitle: t('medicines.form.edit_subtitle', name: @medicine.name)
      ), status: :unprocessable_content
    end
  end

  def destroy
    authorize @medicine
    @medicine.destroy
    redirect_to medicines_url, notice: t('medicines.deleted')
  end

  def refill
    authorize @medicine, :update?

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

    @medicine.paper_trail_event = "restock (qty: #{quantity}, date: #{restock_date.iso8601})"
    @medicine.restock!(quantity: quantity)

    redirect_back_or_to @medicine, notice: t('medicines.refilled')
  rescue ActiveRecord::RecordInvalid => e
    render_refill_error(e.record.errors.full_messages.to_sentence)
  end

  def finder
    authorize Medicine
    render Components::Medicines::FinderView.new
  end

  def search
    authorize Medicine, :finder?
    query = params[:q].to_s.strip
    result = NhsDmd::Search.new.call(query)

    if result.success?
      render json: { results: result.results.map(&:to_h) }
    else
      render json: { results: [], error: result.error }
    end
  end

  def dosages
    medicine = policy_scope(Medicine).find(params[:id])
    authorize medicine
    render json: medicine.dosages.select(:id, :amount, :unit, :description)
  end

  private

  def set_medicine
    @medicine = policy_scope(Medicine).find(params[:id])
  end

  def available_locations
    Location.order(:name)
  end

  def medicine_params
    params.expect(
      medicine: %i[name
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
    render Components::Medicines::ShowView.new(medicine: @medicine, notice: message), status: :unprocessable_content
  end
end
