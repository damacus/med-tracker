# frozen_string_literal: true

class DoseAmount
  def initialize(amount, unit)
    @amount = amount
    @unit = unit
  end

  def to_s
    return '' if @amount.blank? || @unit.blank?

    "#{formatted_value} #{@unit}"
  end

  private

  def formatted_value
    @amount.to_f.to_s.sub(/\.0$/, '')
  end
end
