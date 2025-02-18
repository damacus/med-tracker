class DosageOption < ApplicationRecord
  belongs_to :medicine

  validates :amount, presence: true, numericality: { greater_than: 0 }
  validates :amount, uniqueness: { scope: :medicine_id }

  scope :ordered, -> { order(amount: :asc) }
end
