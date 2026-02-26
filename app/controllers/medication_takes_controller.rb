# frozen_string_literal: true

class MedicationTakesController < ApplicationController
  before_action :set_schedule, only: [:create]

  def create
    # SECURITY: Enforce timing and stock restrictions server-side
    # This prevents bypassing UI-disabled buttons via direct API calls
    unless @schedule.can_administer?
      reason = @schedule.administration_blocked_reason
      message = reason == :out_of_stock ? 'Cannot take medication: out of stock' : 'Cannot take medication: timing restrictions not met'
      respond_to do |format|
        format.html { redirect_back_or_to(root_path, alert: message) }
        format.json { render json: { success: false, errors: [message] }, status: :unprocessable_content }
      end
      return
    end

    @medication_take = @schedule.medication_takes.build(medication_take_params)
    @medication_take.amount_ml ||= @schedule.dosage.amount
    authorize @medication_take

    if @medication_take.save
      respond_to do |format|
        format.html { redirect_back_or_to(root_path, notice: t('take_medications.success')) }
        format.json { render json: { success: true, message: t('take_medications.json_success') } }
      end
    else
      respond_to do |format|
        format.html { redirect_back_or_to(root_path, alert: t('take_medications.failure')) }
        format.json do
          render json: { success: false, errors: @medication_take.errors.full_messages }, status: :unprocessable_content
        end
      end
    end
  end

  private

  def set_schedule
    @schedule = policy_scope(Schedule).find(params[:schedule_id])
  end

  def medication_take_params
    if params[:medication_take].present?
      params.expect(medication_take: %i[taken_at notes]).tap do |whitelisted|
        whitelisted[:taken_at] ||= Time.current
      end
    else
      { taken_at: Time.current }
    end
  end
end
