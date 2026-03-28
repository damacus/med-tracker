# frozen_string_literal: true

# Encapsulates the domain logic for recording a medication dose.
#
# Both Schedule and PersonMedication can be the source of a dose. This service
# handles the shared flow so controllers remain thin.
#
# @example
#   result = TakeMedicationService.new.call(
#     source: @schedule,
#     amount_override: params[:amount_ml],
#     taken_from_medication_id: params[:taken_from_medication_id],
#     user: current_user,
#     taken_at: params[:taken_at] || Time.current   # optional, defaults to now
#   )
#   result.success  # => true / false
#   result.take     # => MedicationTake record (when successful)
#   result.error    # => :out_of_stock | :cooldown | :invalid_amount |
#                   #    :selection_required | :invalid_source | :create_failed
class TakeMedicationService
  Result = Data.define(:success, :take, :error)

  def call(source:, amount_override:, taken_from_medication_id:, user:, taken_at: Time.current)
    resolver = MedicationStockSourceResolver.new(user: user, source: source)

    blocked = resolver.blocked_reason
    return Result.new(success: false, take: nil, error: blocked) if blocked

    amount = normalize_amount(amount_override.presence || source.default_dose_amount)
    return Result.new(success: false, take: nil, error: :invalid_amount) if invalid_amount?(amount)

    if resolver.selection_required?(taken_from_medication_id)
      return Result.new(success: false, take: nil, error: :selection_required)
    end

    medication = resolver.resolve_selected(taken_from_medication_id)
    return Result.new(success: false, take: nil, error: :invalid_source) if medication.blank?

    take = source.medication_takes.create(
      taken_at: taken_at,
      amount_ml: amount,
      taken_from_medication: medication,
      taken_from_location: medication.location
    )

    if take.persisted?
      Result.new(success: true, take: take, error: nil)
    else
      Result.new(success: false, take: nil, error: :create_failed)
    end
  end

  private

  def normalize_amount(raw)
    return nil if raw.blank?

    BigDecimal(raw.to_s)
  rescue ArgumentError
    nil
  end

  def invalid_amount?(amount)
    amount.nil? || amount <= 0
  end
end
