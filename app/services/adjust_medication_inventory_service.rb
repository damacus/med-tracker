# frozen_string_literal: true

class AdjustMedicationInventoryService
  Result = Data.define(:success, :medication, :error) do
    def success?
      success
    end
  end

  def call(medication:, new_quantity:, reason: nil)
    normalized_quantity = normalize_quantity(new_quantity)

    return failure(medication, "Quantity must be a valid number") if normalized_quantity.nil?
    return failure(medication, "Quantity cannot be negative") if normalized_quantity.negative?

    medication.paper_trail_event = build_event_string(normalized_quantity, reason)

    medication.with_lock do
      medication.update!(current_supply: normalized_quantity)
    end

    Result.new(success: true, medication: medication, error: nil)
  rescue ActiveRecord::RecordInvalid => e
    failure(medication, e.record.errors.full_messages.to_sentence)
  end

  private

  def build_event_string(quantity, reason)
    event = "adjust inventory (qty: #{MedicationStockQuantityFormatter.format(quantity)}"
    event += ", reason: #{reason}" if reason.present?
    "#{event})"
  end

  def normalize_quantity(quantity)
    BigDecimal(quantity.to_s)
  rescue ArgumentError
    nil
  end

  def failure(medication, error)
    Result.new(success: false, medication: medication, error: error)
  end
end
