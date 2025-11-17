# frozen_string_literal: true

class MedicationTakesController < ApplicationController
  include Pundit::Authorization

  before_action :set_prescription, only: [:create]

  def create
    @medication_take = @prescription.medication_takes.build(medication_take_params)
    authorize @medication_take

    if @medication_take.save
      respond_to do |format|
        format.html { redirect_back_or_to(root_path, notice: t('take_medicines.success')) }
        format.json { render json: { success: true, message: 'Medication taken successfully recorded.' } }
      end
    else
      respond_to do |format|
        format.html { redirect_back_or_to(root_path, alert: t('take_medicines.failure')) }
        format.json do
          render json: { success: false, errors: @medication_take.errors.full_messages }, status: :unprocessable_content
        end
      end
    end
  end

  private

  def set_prescription
    @prescription = policy_scope(Prescription).find(params[:prescription_id])
  end

  def medication_take_params
    params.expect(medication_take: %i[taken_at notes]).tap do |whitelisted|
      whitelisted[:taken_at] ||= Time.current
    end
  end
end
