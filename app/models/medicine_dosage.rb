# frozen_string_literal: true

class MedicineDosage < ApplicationRecord
  belongs_to :medicine

  validates :amount, presence: true, numericality: { greater_than: 0 }
  validates :unit, presence: true
  validates :description, presence: true
  validates :is_default, inclusion: { in: [true, false] }

  # Ensure only one default dosage per medicine
  validate :single_default_per_medicine

  scope :default_first, -> { order(is_default: :desc, amount: :asc) }

  private

  def single_default_per_medicine
    return unless is_default?
    return if medicine.nil?

    existing_default = medicine.medicine_dosages.where(is_default: true)
    existing_default = existing_default.where.not(id: id) if persisted?

    return unless existing_default.exists?

    errors.add(:is_default, 'can only be set for one dosage per medicine')
  end
end
