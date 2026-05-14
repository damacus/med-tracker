# frozen_string_literal: true

class DoseAmount
  PLURALIZABLE_UNITS = %w[tablet capsule gummy spray drop sachet pad].freeze

  def self.pluralize_unit(amount, unit)
    return unit unless PLURALIZABLE_UNITS.include?(unit)

    amount.to_d == 1 ? unit : unit.pluralize
  end

  def initialize(amount, unit)
    @amount = amount
    @unit = unit
  end

  def label
    return nil if @amount.blank? || @unit.blank?

    "#{formatted_amount} #{self.class.pluralize_unit(@amount, @unit)}"
  end

  private

  def formatted_amount
    value = BigDecimal(@amount.to_s)
    return value.to_i.to_s if value.frac.zero?

    value.to_s('F').sub(/0+\z/, '').sub(/\.\z/, '')
  end
end
