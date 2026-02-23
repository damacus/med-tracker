# frozen_string_literal: true

class Medicine < ApplicationRecord # :nodoc:
  DOSAGE_UNITS = %w[tablet mg ml g mcg IU spray drop sachet].freeze

  belongs_to :location

  has_many :dosages, dependent: :destroy
  has_many :prescriptions, dependent: :destroy
  has_many :person_medicines, dependent: :destroy

  validates :name, presence: true
  validates :dosage_unit, inclusion: { in: DOSAGE_UNITS }, allow_blank: true
  validates :current_supply, numericality: { only_integer: true, greater_than_or_equal_to: 0 }, allow_nil: true
  validates :stock, numericality: { only_integer: true, greater_than_or_equal_to: 0 }, allow_nil: true
  validates :reorder_threshold, presence: true, numericality: { only_integer: true, greater_than_or_equal_to: 0 }

  def low_stock?
    return false if current_supply.nil?

    current_supply <= reorder_threshold
  end

  def out_of_stock?
    return false if current_supply.nil?

    current_supply <= 0
  end
end
