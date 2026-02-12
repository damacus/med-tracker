# frozen_string_literal: true

class MedicineAdministrationService
  Result = Struct.new(:success, :medication_take, :error, :message, keyword_init: true) do
    def success?
      success
    end

    def failure?
      !success
    end
  end

  def self.call(takeable:, amount_ml: nil)
    new(takeable: takeable, amount_ml: amount_ml).call
  end

  def initialize(takeable:, amount_ml: nil)
    @takeable = takeable
    @amount_ml = amount_ml
  end

  def call
    return blocked_result unless takeable.can_administer?

    take = takeable.medication_takes.create!(
      taken_at: Time.current,
      amount_ml: resolved_amount
    )

    Result.new(success: true, medication_take: take)
  end

  private

  attr_reader :takeable, :amount_ml

  def resolved_amount
    amount_ml || default_amount
  end

  def default_amount
    if takeable.respond_to?(:dosage)
      takeable.dosage.amount
    else
      takeable.medicine.dosage_amount
    end
  end

  def blocked_result
    reason = takeable.administration_blocked_reason

    case reason
    when :out_of_stock
      Result.new(success: false, error: :out_of_stock, message: 'Cannot take medicine: out of stock')
    else
      Result.new(success: false, error: :timing_restriction,
                 message: 'Cannot take medicine: timing restrictions not met')
    end
  end
end
