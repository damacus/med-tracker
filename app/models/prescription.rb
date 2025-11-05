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

  def can_take_now?
    return true unless timing_restrictions?

    !would_violate_restrictions?(Time.current)
  end

  def next_available_time
    return nil unless timing_restrictions?
    return Time.current if can_take_now?

    calculate_next_available_time
  end

  private

  def would_violate_restrictions?(check_time)
    would_exceed_max_doses?(check_time) || would_violate_min_hours?(check_time)
  end

  def would_exceed_max_doses?(check_time)
    return false if max_daily_doses.blank?

    doses_today = medication_takes.where(taken_at: check_time.all_day).count

    doses_today >= max_daily_doses
  end

  def would_violate_min_hours?(check_time)
    return false if min_hours_between_doses.blank?

    last_take = medication_takes.where(taken_at: ...check_time).order(taken_at: :desc).first

    return false if last_take.blank?

    hours_since_last = (check_time - last_take.taken_at) / 1.hour
    hours_since_last < min_hours_between_doses
  end

  def calculate_next_available_time
    times = []

    # Check when min hours restriction would be satisfied
    if min_hours_between_doses.present?
      last_take = medication_takes.order(taken_at: :desc).first
      times << (last_take.taken_at + min_hours_between_doses.hours) if last_take
    end

    # Check when max doses restriction would be satisfied (next day)
    times << (Time.current.end_of_day + 1.second) if max_daily_doses.present? && would_exceed_max_doses?(Time.current)

    times.compact.min
  end

  def end_date_after_start_date
    return if end_date.blank? || start_date.blank?

    return unless end_date < start_date

    errors.add(:end_date, 'must be after the start date')
  end
end
