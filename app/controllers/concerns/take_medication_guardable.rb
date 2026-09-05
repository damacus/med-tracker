# frozen_string_literal: true

module TakeMedicationGuardable
  extend ActiveSupport::Concern

  FUTURE_TOLERANCE = MedicationAdministration::RecordDose::FUTURE_TOLERANCE

  private

  # Maps a MedicationAdministration::RecordDose error symbol to the appropriate HTTP response.
  def handle_take_medication_failure(error, scope:)
    case error
    when :future_taken_at
      respond_take_medication_error(message: t('take_medications.future_taken_at'))
    when :out_of_stock, :cooldown, :inactive, :paused, :overlapping_prescription_restriction
      default_message = take_medication_default_message(error)
      respond_take_medication_error(message: t("#{scope}.cannot_take_medication", default: default_message))
    when :invalid_amount, :create_failed, :household_unavailable
      respond_take_medication_invalid_dose(scope:)
    when :selection_required, :invalid_source
      respond_take_medication_stock_source_error(scope:, error:)
    end
  end

  def take_medication_default_message(error)
    case error
    when :out_of_stock then 'Cannot take medication: out of stock'
    when :paused then 'Cannot take medication: paused'
    when :overlapping_prescription_restriction then t('take_medications.overlapping_prescription_restriction')
    else 'Cannot take medication: timing restrictions not met'
    end
  end

  def respond_take_medication_invalid_dose(scope:)
    message = t("#{scope}.invalid_dose_configured")
    respond_take_medication_error(message:)
  end

  def respond_take_medication_stock_source_error(scope:, error:)
    message = if error == :selection_required
                t("#{scope}.location_required", default: 'Choose a location to record this dose.')
              else
                t("#{scope}.invalid_location", default: 'Selected location is unavailable for this medication.')
              end
    respond_take_medication_error(message:)
  end

  def respond_take_medication_error(message:)
    respond_to do |format|
      format.html { redirect_back_or_to(respond_take_medication_redirect_path, alert: message) }
      format.json { render json: { success: false, errors: [message] }, status: :unprocessable_content }
      format.turbo_stream do
        flash.now[:alert] = message
        render turbo_stream: turbo_stream.update('flash', Components::Layouts::Flash.new(alert: flash[:alert]))
      end
    end
  end

  def medication_taken_at_or_respond(scope:)
    taken_at = parsed_medication_taken_at

    if taken_at.blank?
      respond_take_medication_error(
        message: t("#{scope}.invalid_taken_at", default: t('take_medications.invalid_taken_at'))
      )
      return
    end

    if taken_at > Time.current + FUTURE_TOLERANCE
      respond_take_medication_error(
        message: t("#{scope}.future_taken_at", default: t('take_medications.future_taken_at'))
      )
      return
    end

    taken_at
  end

  def log_invalid_take_attempt(**)
    Observability::DiagnosticEvent.emit(
      component: :medication_guard,
      reason: :invalid_payload,
      severity: :warn
    )
  end

  def requested_taken_from_medication_id
    params[:taken_from_medication_id].presence || params.dig(:medication_take, :taken_from_medication_id).presence
  end

  def parsed_medication_taken_at
    raw_taken_at = params.dig(:medication_take, :taken_at).presence
    return Time.current if raw_taken_at.blank?

    return parse_time_only_taken_at(raw_taken_at) if raw_taken_at.match?(/\A\d{2}:\d{2}(:\d{2})?\z/)

    medication_taken_at_formats.each do |format|
      return Time.zone.strptime(raw_taken_at, format)
    rescue ArgumentError, TypeError
      next
    end

    nil
  end

  def parse_time_only_taken_at(raw)
    format = raw.length == 5 ? '%H:%M' : '%H:%M:%S'
    parsed = Time.zone.strptime(raw, format)
    today = Date.current
    Time.zone.local(today.year, today.month, today.day, parsed.hour, parsed.min, parsed.sec)
  rescue ArgumentError, TypeError
    nil
  end

  def respond_take_medication_redirect_path
    defined?(@person) && @person.present? ? person_path(@person) : root_path
  end

  def medication_taken_at_formats
    ['%Y-%m-%dT%H:%M', '%Y-%m-%dT%H:%M:%S']
  end
end
