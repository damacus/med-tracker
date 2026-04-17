# frozen_string_literal: true

class SupplyLevel
  attr_reader :raw_current, :reorder_threshold, :last_restock

  def initialize(current:, reorder_threshold:, last_restock:)
    @raw_current = current
    @reorder_threshold = reorder_threshold.to_i
    @last_restock = last_restock
  end

  def current
    raw_current || 0
  end

  def tracked?
    !raw_current.nil?
  end

  def percentage
    denominator = last_restock || [reorder_threshold, 1].max
    return 0 if denominator <= 0

    [(current.to_f / denominator * 100), 100].min.round
  end

  def low_stock?
    return false unless tracked?

    current <= reorder_threshold
  end

  def crossed_low_stock_threshold_from?(previous_current:)
    return false unless tracked?
    return false if previous_current.nil?

    previous_current.to_i > reorder_threshold && current <= reorder_threshold
  end

  def out_of_stock?
    return false unless tracked?

    current <= 0
  end

  def status
    return :out_of_stock if out_of_stock?
    return :low_stock if low_stock?

    :in_stock
  end

  def days_until_low_stock(daily_consumption:)
    return nil unless forecast_available?(daily_consumption:)
    return 0 if low_stock?

    surplus = current - reorder_threshold
    return 0 if surplus <= 0

    (surplus.to_f / daily_consumption).ceil
  end

  def days_until_out_of_stock(daily_consumption:)
    return nil unless forecast_available?(daily_consumption:)
    return 0 if out_of_stock?

    (current.to_f / daily_consumption).ceil
  end

  def forecast_available?(daily_consumption:)
    tracked? && daily_consumption.to_f.positive?
  end
end
