# frozen_string_literal: true

class DoseAmount
  PLURALIZABLE_UNITS = %w[tablet capsule spray drop sachet pad].freeze
  IRREGULAR_PLURAL_UNITS = { 'gummy' => 'gummies' }.freeze

  def self.pluralize_unit(amount, unit)
    return unit unless PLURALIZABLE_UNITS.include?(unit) || IRREGULAR_PLURAL_UNITS.key?(unit)

    amount.to_d == 1 ? unit : IRREGULAR_PLURAL_UNITS.fetch(unit, "#{unit}s")
  end

  def initialize(amount, unit)
    @amount = amount
    @unit = unit
  end

  def to_s
    return '' if @amount.blank? || @unit.blank?

    "#{formatted_value} #{self.class.pluralize_unit(@amount, @unit)}"
  end

  private

  def formatted_value
    @amount.to_f.to_s.sub(/\.0$/, '')
  end
end
