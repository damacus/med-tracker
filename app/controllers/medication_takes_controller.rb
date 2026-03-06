# frozen_string_literal: true

class MedicationTakesController < ApplicationController
  include TakeMedicationGuardable

  before_action :set_schedule, only: [:create]

  def create
    if (reason = take_medication_blocked_reason(@schedule))
      respond_create_error(message: blocked_reason_message(reason))
      return
    end

    stock_source_error, taken_from_medication = resolve_taken_from_medication(@schedule)
    if stock_source_error
      respond_create_error(message: stock_source_error_message(stock_source_error))
      return
    end

    @medication_take = @schedule.medication_takes.build(
      medication_take_params.merge(
        taken_from_medication: taken_from_medication,
        taken_from_location: taken_from_medication.location
      )
    )
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
      params.expect(medication_take: %i[taken_at notes taken_from_medication_id]).tap do |whitelisted|
        whitelisted[:taken_at] ||= Time.current
      end
    else
      {
        taken_at: Time.current,
        taken_from_medication_id: requested_taken_from_medication_id
      }
    end
  end

  def blocked_reason_message(reason)
    reason == :out_of_stock ? 'Cannot take medication: out of stock' : 'Cannot take medication: timing restrictions not met'
  end

  def stock_source_error_message(stock_source_error)
    if stock_source_error == :selection_required
      'Choose a location to record this dose.'
    else
      'Selected location is unavailable for this medication.'
    end
  end

  def respond_create_error(message:)
    respond_to do |format|
      format.html { redirect_back_or_to(root_path, alert: message) }
      format.json { render json: { success: false, errors: [message] }, status: :unprocessable_content }
    end
  end
end
