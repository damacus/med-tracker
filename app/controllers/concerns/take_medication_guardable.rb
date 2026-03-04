# frozen_string_literal: true

module TakeMedicationGuardable
  extend ActiveSupport::Concern

  private

  def invalid_take_amount?(amount)
    amount.nil? || amount <= 0
  end

  def normalized_take_amount(raw_amount)
    return nil if raw_amount.blank?

    BigDecimal(raw_amount.to_s)
  rescue ArgumentError
    nil
  end

  def respond_take_medication_invalid_dose(scope:)
    message = t("#{scope}.invalid_dose_configured")
    respond_to do |format|
      format.html { redirect_back_or_to person_path(@person), alert: message }
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
end
