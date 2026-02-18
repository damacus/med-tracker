# frozen_string_literal: true

class MedicinesController < ApplicationController
  include Pundit::Authorization

  before_action :set_medicine, only: %i[show edit update destroy]

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
      title: t('medicines.form.new_title'),
      subtitle: t('medicines.form.new_subtitle')
    )
  end

  def edit
    authorize @medicine
    render Components::Medicines::FormView.new(
      medicine: @medicine,
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

  def medicine_params
    params.expect(
      medicine: %i[name
                   description
                   dosage_amount
                   dosage_unit
                   current_supply
                   stock
                   warnings]
    )
  end
end
