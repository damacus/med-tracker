# frozen_string_literal: true

class MedicationTakesController < ApplicationController
  before_action :set_prescription, only: [:create]

  def create
    @medication_take = @prescription.medication_takes.build(medication_take_params)

    if @medication_take.save
      respond_to do |format|
        format.html { redirect_back(fallback_location: root_path, notice: 'Medication taken successfully recorded.') }
        format.json { render json: { success: true, message: 'Medication taken successfully recorded.' } }
      end
    else
      respond_to do |format|
        format.html { redirect_back(fallback_location: root_path, alert: 'Failed to record medication taken.') }
        format.json do
          render json: { success: false, errors: @medication_take.errors.full_messages }, status: :unprocessable_entity
        end
      end
    end
  end

  private

  def set_prescription
    @prescription = Prescription.find(params[:prescription_id])
  end

  def medication_take_params
    params.require(:medication_take).permit(:taken_at, :notes).tap do |whitelisted|
      whitelisted[:taken_at] ||= Time.current
    end
  end
end
