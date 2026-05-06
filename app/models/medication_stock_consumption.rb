# frozen_string_literal: true

class MedicationStockConsumption
  VOLUME_UNITS = %w[ml].freeze

  def self.quantity_for(dose_amount:, dose_unit:)
    return BigDecimal('0') if dose_amount.blank?
    return BigDecimal(dose_amount.to_s) if volume_unit?(dose_unit)

    BigDecimal('1')
  end

  def self.sufficient?(current_supply:, dose_amount:, dose_unit:)
    return true if current_supply.nil?

    BigDecimal(current_supply.to_s) >= quantity_for(dose_amount: dose_amount, dose_unit: dose_unit)
  end

  def self.volume_unit?(unit)
    VOLUME_UNITS.include?(unit.to_s)
  end

  def self.format(quantity)
    decimal = BigDecimal(quantity.to_s)
    value = decimal.frac.zero? ? decimal.to_i.to_s : decimal.to_s('F')

    value.include?('.') ? value.sub(/\.?0+\z/, '') : value
  end
end
