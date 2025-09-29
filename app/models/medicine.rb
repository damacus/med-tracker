# frozen_string_literal: true

class Medicine < ApplicationRecord # :nodoc:
  has_many :dosages, dependent: :destroy
  has_many :prescriptions, dependent: :destroy

  validates :name, presence: true
  validates :current_supply, presence: true, numericality: { only_integer: true, greater_than_or_equal_to: 0 }
  validates :stock, presence: true, numericality: { only_integer: true, greater_than_or_equal_to: 0 }
  validates :reorder_threshold, presence: true, numericality: { only_integer: true, greater_than: 0 }

  def low_stock?
    return false if stock.blank? || reorder_threshold.blank?

    stock < reorder_threshold
  end
end
