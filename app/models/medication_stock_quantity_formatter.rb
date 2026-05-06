# frozen_string_literal: true

class MedicationStockQuantityFormatter
  def self.format(quantity)
    new(quantity).format
  end

  def initialize(quantity)
    @quantity = quantity
  end

  def format
    value.include?('.') ? value.sub(/\.?0+\z/, '') : value
  end

  private

  attr_reader :quantity

  def value
    decimal.frac.zero? ? decimal.to_i.to_s : decimal.to_s('F')
  end

  def decimal
    @decimal ||= BigDecimal(quantity.to_s)
  end
end
