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
    return failure(resolver.blocked_reason) if resolver.blocked_reason

    amount = normalize_amount(amount_override.presence || default_dose_amount_for(source, taken_at))
    return failure(:invalid_amount) if invalid_amount?(amount)

    error, medication = resolve_stock_source(resolver, taken_from_medication_id)
    return failure(error) if error

    take = record_take(source: source, amount: amount, medication: medication, taken_at: taken_at)
    return failure(:create_failed) unless take.persisted?

    success(take)
  end

  private

  def failure(error)
    Result.new(success: false, take: nil, error: error)
  end

  def success(take)
    publish_dose_taken(take)
    Result.new(success: true, take: take, error: nil)
  end

  def resolve_stock_source(resolver, taken_from_medication_id)
    return [:selection_required, nil] if resolver.selection_required?(taken_from_medication_id)

    medication = resolver.resolve_selected(taken_from_medication_id)
    return [:invalid_source, nil] if medication.blank?

    [nil, medication]
  end

  def record_take(source:, amount:, medication:, taken_at:)
    source.medication_takes.create(
      taken_at: taken_at,
      amount_ml: amount,
      taken_from_medication: medication,
      taken_from_location: medication.location
    )
  end

  def normalize_amount(raw)
    return nil if raw.blank?

    BigDecimal(raw.to_s)
  rescue ArgumentError
    nil
  end

  def default_dose_amount_for(source, taken_at)
    return source.effective_dose_amount(effective_date(taken_at)) if source.respond_to?(:effective_dose_amount)

    source.default_dose_amount
  end

  def effective_date(taken_at)
    return taken_at.to_date if taken_at.respond_to?(:to_date)

    Time.zone.today
  end

  def invalid_amount?(amount)
    amount.nil? || amount <= 0
  end

  def publish_dose_taken(take)
    ActiveSupport::Notifications.instrument('dose_taken.med_tracker', dose_taken_payload(take))
  end

  def dose_taken_payload(take)
    {
      take_id: take.id,
      source_type: take_source_type(take),
      source_id: take_source_id(take),
      person_id: take.person&.id,
      medication_id: take.medication&.id,
      inventory_medication_id: take.inventory_medication&.id,
      inventory_location_id: take.inventory_location&.id,
      amount_ml: take.amount_ml&.to_f,
      taken_at: take.taken_at
    }
  end

  def take_source_type(take)
    take.schedule_id.present? ? 'schedule' : 'person_medication'
  end

  def take_source_id(take)
    take.schedule_id || take.person_medication_id
  end
end
