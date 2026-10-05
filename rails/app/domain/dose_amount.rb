# frozen_string_literal: true

# typed: true

require 'sorbet-runtime'

class DoseAmount
  extend T::Sig

  PLURALIZABLE_UNITS = T.let(%w[tablet capsule spray drop sachet pad].freeze, T::Array[String])

  sig { params(amount: T.untyped, unit: T.untyped).returns(T.untyped).checked(:never) }
  def self.pluralize_unit(amount, unit)
    return unit unless PLURALIZABLE_UNITS.include?(unit)

    amount.to_d == 1 ? unit : "#{unit}s"
  end

  sig { params(amount: T.untyped, unit: T.untyped).void.checked(:never) }
  def initialize(amount, unit)
    @amount = amount
    @unit = unit
  end

  sig { returns(String) }
  def to_s
    return '' if @amount.blank? || @unit.blank?

    "#{formatted_value} #{self.class.pluralize_unit(@amount, @unit)}"
  end

  private

  sig { returns(String) }
  def formatted_value
    @amount.to_f.to_s.sub(/\.0$/, '')
  end
end
