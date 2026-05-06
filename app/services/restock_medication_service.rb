# frozen_string_literal: true

class RestockMedicationService
  Result = Data.define(:success, :medication, :error) do
    def success?
      success
    end
  end

  def call(medication:, quantity:, restock_date:)
    normalized_quantity = normalize_quantity(quantity)
    normalized_date = normalize_date(restock_date)

    if normalized_quantity.blank? || normalized_quantity <= 0
      return failure(medication, 'Quantity must be greater than 0')
    end
    return failure(medication, 'Restock date is invalid') unless normalized_date

    medication.paper_trail_event = "restock (qty: #{MedicationStockQuantityFormatter.format(normalized_quantity)}, " \
                                   "date: #{normalized_date.iso8601})"
    medication.restock!(quantity: normalized_quantity)

    Result.new(success: true, medication: medication, error: nil)
  rescue ActiveRecord::RecordInvalid => e
    failure(medication, e.record.errors.full_messages.to_sentence)
  end

  private

  def normalize_quantity(quantity)
    BigDecimal(quantity.to_s)
  rescue ArgumentError
    nil
  end

  def normalize_date(restock_date)
    return restock_date if restock_date.respond_to?(:iso8601)

    Date.parse(restock_date.to_s)
  rescue ArgumentError
    nil
  end

  def failure(medication, error)
    Result.new(success: false, medication: medication, error: error)
  end
end
