class Medicine < ApplicationRecord
  has_many :dosages, dependent: :destroy
  has_many :prescriptions, dependent: :destroy

  validates :name, presence: true
  validates :current_supply, presence: true, numericality: { only_integer: true, greater_than_or_equal_to: 0 }
end
