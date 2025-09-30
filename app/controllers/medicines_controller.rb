# frozen_string_literal: true

class MedicinesController < ApplicationController
  before_action :set_medicine, only: %i[show edit update destroy]

  def index
    medicines = Medicine.all
    render Components::Medicines::IndexView.new(medicines: medicines)
  end

  def show
    render Components::Medicines::ShowView.new(medicine: @medicine, notice: flash[:notice])
  end

  def new
    @medicine = Medicine.new
    render Components::Medicines::FormView.new(
      medicine: @medicine,
      title: 'Add a New Medicine',
      subtitle: 'Capture inventory details and dosage information.'
    )
  end

  def edit
    render Components::Medicines::FormView.new(
      medicine: @medicine,
      title: 'Edit Medicine',
      subtitle: "Update #{@medicine.name}'s details."
    )
  end

  def create
    @medicine = Medicine.new(medicine_params)

    if @medicine.save
      redirect_to @medicine, notice: 'Medicine was successfully created.'
    else
      render Components::Medicines::FormView.new(
        medicine: @medicine,
        title: 'Add a New Medicine',
        subtitle: 'Capture inventory details and dosage information.'
      ), status: :unprocessable_entity
    end
  end

  def update
    if @medicine.update(medicine_params)
      redirect_to @medicine, notice: 'Medicine was successfully updated.'
    else
      render Components::Medicines::FormView.new(
        medicine: @medicine,
        title: 'Edit Medicine',
        subtitle: "Update #{@medicine.name}'s details."
      ), status: :unprocessable_entity
    end
  end

  def destroy
    @medicine.destroy
    redirect_to medicines_url, notice: 'Medicine was successfully deleted.'
  end

  def finder
    render Components::Medicines::FinderView.new
  end

  private

  def set_medicine
    @medicine = Medicine.find(params[:id])
  end

  def medicine_params
    params.require(:medicine).permit(
      :name,
      :description,
      :dosage_amount,
      :dosage_unit,
      :current_supply,
      :stock,
      :warnings
    )
  end
end
