class Prescription < ApplicationRecord
  belongs_to :person
  belongs_to :medicine
  has_many :take_medicines, dependent: :destroy

  validates :dosage, presence: true
  validates :frequency, presence: true
  validates :start_date, presence: true
  validate :end_date_after_start_date, if: -> { end_date.present? }
  validates :max_daily_doses, numericality: { greater_than: 0, allow_nil: true }
  validates :min_hours_between_doses, numericality: { greater_than: 0, allow_nil: true }
  validates :dose_cycle, inclusion: { in: %w[daily weekly monthly], allow_blank: true }

  scope :active, -> { where("end_date IS NULL OR end_date >= ?", Date.current) }
  scope :inactive, -> { where("end_date < ?", Date.current) }

  # List of valid dose cycles
  DOSE_CYCLES = {
    "daily" => 1.day,
    "weekly" => 1.week,
    "monthly" => 1.month
  }.freeze

  def active?
    end_date.nil? || end_date >= Date.current
  end

  def recommended_dosage
    medicine.find_recommended_dosage(person.age)
  end

  def total_ml_taken_today
    take_medicines.total_ml_today
  end

  def total_ml_taken_24h
    take_medicines.total_ml_24h
  end

  def max_daily_ml
    if recommended = recommended_dosage
      recommended.amount_ml * recommended.frequency_per_day
    else
      nil # No recommended dosage found for this age
    end
  end

  def remaining_ml_allowed
    if max = max_daily_ml
      take_medicines.remaining_ml_allowed(max)
    else
      nil # No recommended dosage found for this age
    end
  end

  # Check if a new dose can be taken now based on timing restrictions
  def can_take_dose_now?
    return true unless has_timing_restrictions?

    return false unless passes_max_daily_doses?
    return false unless passes_min_hours_between_doses?

    true
  end

  # Returns true if this prescription has any timing restrictions
  def has_timing_restrictions?
    max_daily_doses.present? || min_hours_between_doses.present? || dose_cycle.present?
  end

  # Check if another dose would exceed the maximum daily doses
  def passes_max_daily_doses?
    return true unless max_daily_doses.present?

    doses_taken_in_cycle = doses_taken_in_current_cycle
    doses_taken_in_cycle < max_daily_doses
  end

  # Check if enough time has passed since the last dose
  def passes_min_hours_between_doses?
    return true unless min_hours_between_doses.present?

    last_dose = take_medicines.recent.first
    return true unless last_dose # No previous dose

    hours_since_last_dose = ((Time.current - last_dose.taken_at) / 1.hour).round
    hours_since_last_dose >= min_hours_between_doses
  end

  # Get the number of doses taken in the current cycle (day/week/month)
  def doses_taken_in_current_cycle
    cycle_period = get_cycle_period

    if cycle_period == 1.day
      take_medicines.today.count
    else
      # Default to daily if no cycle specified
      cycle_start = Time.current.beginning_of_day - cycle_period + 1.day
      take_medicines.where(taken_at: cycle_start..Time.current).count
    end
  end

  # Get the appropriate time period based on dose_cycle
  def get_cycle_period
    return 1.day if dose_cycle.blank?

    DOSE_CYCLES[dose_cycle] || 1.day
  end

  # Time until next dose is available
  def time_until_next_dose
    return nil unless has_timing_restrictions?
    return nil if can_take_dose_now?

    if !passes_min_hours_between_doses? && min_hours_between_doses.present?
      last_dose = take_medicines.recent.first
      next_available = last_dose.taken_at + min_hours_between_doses.hours
      seconds_remaining = (next_available - Time.current).round
      return seconds_remaining if seconds_remaining > 0
    end

    if !passes_max_daily_doses? && max_daily_doses.present?
      # Calculate when the cycle resets
      cycle_period = get_cycle_period
      cycle_end = Time.current.beginning_of_day + cycle_period
      seconds_remaining = (cycle_end - Time.current).round
      return seconds_remaining if seconds_remaining > 0
    end

    nil
  end

  private

  def end_date_after_start_date
    return if end_date.blank? || start_date.blank?

    if end_date < start_date
      errors.add(:end_date, "must be after the start date")
    end
  end
end
