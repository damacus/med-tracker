# frozen_string_literal: true

# Prescription model
class Prescription < ApplicationRecord
  belongs_to :person
  belongs_to :medicine
  belongs_to :dosage

  enum :dose_cycle, { daily: 0, weekly: 1, monthly: 2 }

  has_many :medication_takes, dependent: :destroy
  alias take_medicines medication_takes

  scope :active, lambda {
    where('start_date <= ? AND end_date >= ?', Time.zone.today, Time.zone.today)
  }

  validates :start_date, presence: true
  validates :end_date, presence: true
  validate :end_date_after_start_date

  def timing_restrictions?
    max_daily_doses.present? || min_hours_between_doses.present?
  end

  def cycle_period
    case dose_cycle
    when 'weekly'
      7.days
    when 'monthly'
      30.days
    else
      1.day # Handles 'daily', nil, and any other values
    end
  end

  private

  def end_date_after_start_date
    return if end_date.blank? || start_date.blank?

    return unless end_date < start_date

    errors.add(:end_date, 'must be after the start date')
  end
end
