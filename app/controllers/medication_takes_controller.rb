# frozen_string_literal: true

class MedicationTakesController < ApplicationController
  include TakeMedicationGuardable

  before_action :set_schedule, only: [:create]

  def create
    authorize(@schedule, :take_medication?)
    taken_at = medication_taken_at_or_respond(scope: "take_medications")
    return unless taken_at

    result = TakeMedicationService.new.call(
      source: @schedule,
      amount_override: nil,
      taken_from_medication_id: requested_taken_from_medication_id,
      user: current_user,
      taken_at: taken_at
    )

    if result.success
      respond_to do |format|
        format.html { redirect_back_or_to(root_path, notice: t("take_medications.success")) }
        format.json { render(json: {success: true, message: t("take_medications.json_success")}) }
      end
    else
      message = failure_message(result.error)
      respond_to do |format|
        format.html { redirect_back_or_to(root_path, alert: message) }
        format.json { render(json: {success: false, errors: [message]}, status: :unprocessable_content) }
      end
    end
  end

  private

  def set_schedule
    @schedule = policy_scope(Schedule).find(params[:schedule_id])
  end

  def failure_message(error)
    case error
    when :out_of_stock
      t("take_medications.out_of_stock", default: "Cannot take medication: out of stock")
    when :cooldown
      t("take_medications.cooldown", default: "Cannot take medication: timing restrictions not met")
    when :selection_required
      t("take_medications.location_required", default: "Choose a location to record this dose.")
    when :invalid_source
      t("take_medications.invalid_location", default: "Selected location is unavailable for this medication.")
    else
      t("take_medications.failure")
    end
  end
end
