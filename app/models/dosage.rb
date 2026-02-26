# frozen_string_literal: true

class Dosage < ApplicationRecord
  belongs_to :medication
  has_many :schedules, dependent: :destroy

  validates :amount, presence: true, numericality: { greater_than: 0 }
  validates :unit, presence: true
  validates :frequency, presence: true
end
