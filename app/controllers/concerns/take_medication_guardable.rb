# frozen_string_literal: true

module TakeMedicationGuardable
  extend ActiveSupport::Concern

  private

  def invalid_take_amount?(amount)
    amount.nil? || amount <= 0
  end

  def take_medication_blocked_reason(source)
    MedicationStockSourceResolver.new(user: current_user, source: source).blocked_reason
  end

  def resolve_taken_from_medication(source)
    resolver = MedicationStockSourceResolver.new(user: current_user, source: source)
    taken_from_medication_id = requested_taken_from_medication_id

    return [:selection_required, nil] if resolver.selection_required?(taken_from_medication_id)

    medication = resolver.resolve_selected(taken_from_medication_id)
    return [:invalid_source, nil] if medication.blank?

    [nil, medication]
  end

  def normalized_take_amount(raw_amount)
    return nil if raw_amount.blank?

    BigDecimal(raw_amount.to_s)
  rescue ArgumentError
    nil
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
      format.turbo_stream do
        flash.now[:alert] = message
        render turbo_stream: turbo_stream.update('flash', Components::Layouts::Flash.new(alert: flash[:alert]))
      end
    end
  end

  def log_invalid_take_attempt(source:, amount:, metadata: {})
    Rails.logger.warn(
      {
        event: 'invalid_take_medication',
        controller: self.class.name,
        source: source,
        person_id: @person.id,
        attempted_amount_ml: amount&.to_s
      }.merge(metadata).to_json
    )
  end

  def requested_taken_from_medication_id
    params[:taken_from_medication_id].presence || params.dig(:medication_take, :taken_from_medication_id).presence
  end

  def respond_take_medication_redirect_path
    defined?(@person) && @person.present? ? person_path(@person) : root_path
  end
end
