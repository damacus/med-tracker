class Dosage < ApplicationRecord
  belongs_to :medicine
  has_many :prescriptions, dependent: :destroy

  validates :amount, presence: true, numericality: { greater_than: 0 }
  validates :unit, presence: true
  validates :frequency, presence: true
end
