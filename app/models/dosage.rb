# frozen_string_literal: true

class Dosage < ApplicationRecord
  belongs_to :medication
  has_many :schedules, dependent: :destroy

  enum :default_dose_cycle, { daily: 0, weekly: 1, monthly: 2 }, prefix: :default

  scope :adult_default,  -> { where(default_for_adults: true) }
  scope :child_default,  -> { where(default_for_children: true) }

  validates :amount, presence: true, numericality: { greater_than: 0 }
  validates :unit, presence: true
  validates :frequency, presence: true
end
