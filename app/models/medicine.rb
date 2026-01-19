# frozen_string_literal: true

class Medicine < ApplicationRecord # :nodoc:
  has_many :dosages, dependent: :destroy
  has_many :prescriptions, dependent: :destroy
  has_many :person_medicines, dependent: :destroy

  validates :name, presence: true
  validates :current_supply, numericality: { only_integer: true, greater_than_or_equal_to: 0 }, allow_nil: true
  validates :stock, numericality: { only_integer: true, greater_than_or_equal_to: 0 }, allow_nil: true
  validates :reorder_threshold, presence: true, numericality: { only_integer: true, greater_than_or_equal_to: 0 }

  def low_stock?
    return false if stock.nil?

    stock <= reorder_threshold
  end

  def out_of_stock?
    stock.to_i <= 0
  end
end
