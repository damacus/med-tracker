# frozen_string_literal: true

class MedicationStockConsumption
  COUNTABLE_UNITS = %w[tablet capsule gummy sachet spray drop pad].freeze
  VOLUME_UNITS = %w[ml].freeze

  def self.quantity_for(dose_amount:, dose_unit:)
    return BigDecimal('0') if dose_amount.blank?
    return BigDecimal(dose_amount.to_s) if stock_quantity_unit?(dose_unit)

    BigDecimal('1')
  end

  def self.sufficient?(current_supply:, dose_amount:, dose_unit:)
    return true if current_supply.nil?

    BigDecimal(current_supply.to_s) >= quantity_for(dose_amount: dose_amount, dose_unit: dose_unit)
  end

  def self.countable_unit?(unit)
    COUNTABLE_UNITS.include?(unit.to_s)
  end

  def self.volume_unit?(unit)
    VOLUME_UNITS.include?(unit.to_s)
  end

  def self.stock_quantity_unit?(unit)
    countable_unit?(unit) || volume_unit?(unit)
  end
end
