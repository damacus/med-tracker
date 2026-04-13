# frozen_string_literal: true

MedicationDosage = Data.define(
  :amount,
  :unit,
  :frequency,
  :description,
  :default_for_adults,
  :default_for_children,
  :default_max_daily_doses,
  :default_min_hours_between_doses,
  :default_dose_cycle
) do
  def default_for_adults?
    default_for_adults
  end

  def default_for_children?
    default_for_children
  end

  def dose_display
    DoseAmount.new(amount, unit).to_s.presence
  end

  def selection_key
    [normalized_amount, unit].join('|')
  end

  def to_option_payload
    {
      selection_key: selection_key,
      amount: normalized_amount,
      unit: unit,
      frequency: frequency,
      description: description,
      default_for_adults: default_for_adults,
      default_for_children: default_for_children,
      default_max_daily_doses: default_max_daily_doses,
      default_min_hours_between_doses: default_min_hours_between_doses,
      default_dose_cycle: default_dose_cycle
    }
  end

  delegate :to_s, to: :dose_display

  private

  def normalized_amount
    return amount.to_i.to_s if amount.is_a?(BigDecimal) && amount.frac.zero?
    return amount.to_s('F') if amount.is_a?(BigDecimal)

    amount.to_s
  end
end

MedicationDosage::DOSE_CYCLE_OPTIONS = [
  %w[Daily daily],
  %w[Weekly weekly],
  %w[Monthly monthly]
].freeze
