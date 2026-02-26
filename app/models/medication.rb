# frozen_string_literal: true

class Medication < ApplicationRecord # :nodoc:
  DOSAGE_UNITS = %w[tablet mg ml g mcg IU spray drop sachet].freeze
  CATEGORIES = %w[painkiller antibiotic vitamin respiratory heart supplement allergy digestive skin].freeze

  has_paper_trail if: proc { |medication| medication.paper_trail_event.present? }

  belongs_to :location

  has_many :dosages, dependent: :destroy
  has_many :schedules, dependent: :destroy
  has_many :person_medications, dependent: :destroy

  enum :reorder_status, { requested: 0, ordered: 1, received: 2 }, prefix: :reorder

  validates :name, presence: true
  validates :category, inclusion: { in: CATEGORIES }, allow_blank: true
  validates :dosage_unit, inclusion: { in: DOSAGE_UNITS }, allow_blank: true
  validates :current_supply, numericality: { only_integer: true, greater_than_or_equal_to: 0 }, allow_nil: true
  validates :supply_at_last_restock, numericality: { only_integer: true, greater_than_or_equal_to: 0 }, allow_nil: true
  validates :reorder_threshold, presence: true, numericality: { only_integer: true, greater_than_or_equal_to: 0 }

  def restock!(quantity:) # rubocop:disable Naming/PredicateMethod
    increment = quantity.to_i
    return false if increment <= 0

    with_lock do
      new_supply = current_supply.to_i + increment
      update!(
        current_supply: new_supply,
        supply_at_last_restock: new_supply,
        reorder_status: nil,
        ordered_at: nil,
        reordered_at: nil
      )
    end

    true
  end

  def supply_percentage
    current = current_supply || 0
    denominator = supply_at_last_restock || [reorder_threshold, 1].max
    return 0 if denominator <= 0

    [current.to_f / denominator * 100, 100].min.round
  end

  def low_stock?
    return false if current_supply.nil?

    current_supply <= reorder_threshold
  end

  def out_of_stock?
    return false if current_supply.nil?

    current_supply <= 0
  end

  def estimated_daily_consumption
    schedule_rate = schedules.active.sum do |schedule|
      next 0.0 if schedule.max_daily_doses.blank?

      schedule.max_daily_doses.to_f / (schedule.cycle_period / 1.day)
    end

    pm_rate = person_medications.sum do |pm|
      next 0.0 if pm.max_daily_doses.blank?

      pm.max_daily_doses.to_f
    end

    schedule_rate + pm_rate
  end

  def forecast_available?
    current_supply.present? && estimated_daily_consumption.positive?
  end

  def days_until_out_of_stock
    return nil unless forecast_available?
    return 0 if out_of_stock?

    (current_supply.to_f / estimated_daily_consumption).ceil
  end

  def days_until_low_stock
    return nil unless forecast_available?
    return 0 if low_stock?

    surplus = current_supply - reorder_threshold
    return 0 if surplus <= 0

    (surplus.to_f / estimated_daily_consumption).ceil
  end

  def out_of_stock_date
    days = days_until_out_of_stock
    days ? Time.zone.today + days.days : nil
  end

  def low_stock_date
    days = days_until_low_stock
    days ? Time.zone.today + days.days : nil
  end
end
