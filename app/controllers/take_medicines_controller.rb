# frozen_string_literal: true

class TakeMedicinesController < ApplicationController
  before_action :set_prescription, only: [:create]

  def create
    # Always use the current time when creating a new take_medicine
    @take_medicine = @prescription.take_medicines.build(take_medicine_params)
    @take_medicine.taken_at ||= Time.current

    if @take_medicine.save
      respond_to do |format|
        format.html { redirect_back(fallback_location: root_path, notice: t('take_medicines.success')) }
        format.json { render json: { success: true, message: 'Medication taken successfully recorded.' } }
      end
    else
      respond_to do |format|
        format.html { redirect_back(fallback_location: root_path, alert: t('take_medicines.failure')) }
        format.json do
          render json: { success: false, errors: @take_medicine.errors.full_messages }, status: :unprocessable_entity
        end
      end
    end
  end

  private

  def set_prescription
    @prescription = Prescription.find(params[:prescription_id])
  end

  def take_medicine_params
    # Only permit the parameters, don't set defaults here
    params.fetch(:take_medicine, {}).permit(:taken_at, :notes)
  end
end
