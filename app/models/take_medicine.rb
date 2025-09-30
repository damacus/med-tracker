# frozen_string_literal: true

class TakeMedicine < ApplicationRecord
  self.table_name = 'medication_takes'

  belongs_to :prescription

  validates :taken_at, presence: true
  validates :amount_ml, presence: true, numericality: { greater_than: 0 }
  validate :taken_at_not_in_future
  validate :check_timing_restrictions

  # Scopes
  scope :today, -> { where(taken_at: Time.current.all_day) }
  scope :this_week, -> { where(taken_at: Time.current.all_week) }
  scope :recent, -> { order(taken_at: :desc) }

  # Delegate methods to access prescription details easily
  delegate :person, :medicine, to: :prescription

  def self.total_ml_today
    today.sum(:amount_ml)
  end

  def self.total_ml_24h
    where(taken_at: 24.hours.ago..Time.current).sum(:amount_ml)
  end

  def self.remaining_ml_allowed(max_daily_ml)
    max_daily_ml - total_ml_24h
  end

  private

  def taken_at_not_in_future
    return unless taken_at.present? && taken_at > Time.current

    errors.add(:taken_at, 'cannot be in the future')
  end

  # Validate against prescription timing restrictions
  def check_timing_restrictions
    return unless prescription&.has_timing_restrictions?
    return if taken_at.blank?

    validate_timing_restrictions(taken_at)
  end

  def validate_timing_restrictions(validation_time = Time.current)
    validate_max_doses_per_cycle(validation_time)
    validate_minimum_time_between_doses(validation_time)
  end

  # Validate that the maximum doses per cycle hasn't been exceeded
  def validate_max_doses_per_cycle(validation_time)
    return if prescription.max_daily_doses.blank?

    existing_doses = count_doses_in_cycle(validation_time)

    return unless existing_doses >= prescription.max_daily_doses

    cycle_name = format_cycle_name(prescription.dose_cycle)
    errors.add(:base, "Maximum of #{prescription.max_daily_doses} doses per #{cycle_name} allowed")
  end

  # Count the number of doses taken in the current cycle
  def count_doses_in_cycle(validation_time)
    cycle_period = prescription.get_cycle_period

    cycle_start = calculate_cycle_start(validation_time, cycle_period)

    prescription.take_medicines
                .where(taken_at: cycle_start..validation_time)
                .where.not(id: id)
                .count
  end

  # Calculate the start of the current cycle
  def calculate_cycle_start(validation_time, cycle_period)
    if cycle_period == 1.day
      validation_time.beginning_of_day
    else
      validation_time.beginning_of_day - cycle_period + 1.day
    end
  end

  # Validate that enough time has passed since the last dose
  def validate_minimum_time_between_doses(validation_time)
    return if prescription.min_hours_between_doses.blank?

    validate_time_since_last_dose(validation_time)
    validate_time_until_next_dose(validation_time) if persisted?
  end

  # Check if enough time has passed since the last dose
  def validate_time_since_last_dose(validation_time)
    last_dose = find_last_dose_before(validation_time)
    return if last_dose.blank?

    hours_since_last_dose = calculate_hours_between(last_dose.taken_at, validation_time)

    return unless hours_since_last_dose < prescription.min_hours_between_doses

    add_minimum_time_error
  end

  # Check if there's enough time until the next dose
  def validate_time_until_next_dose(validation_time)
    next_dose = find_next_dose_after(validation_time)
    return if next_dose.blank?

    hours_until_next_dose = calculate_hours_between(validation_time, next_dose.taken_at)

    return unless hours_until_next_dose < prescription.min_hours_between_doses

    add_minimum_time_error
  end

  # Find the most recent dose before the given time
  def find_last_dose_before(validation_time)
    prescription.take_medicines
                .where.not(id: id)
                .where(taken_at: ...validation_time)
                .order(taken_at: :desc)
                .first
  end

  # Find the next dose after the given time
  def find_next_dose_after(validation_time)
    prescription.take_medicines
                .where.not(id: id)
                .where('taken_at > ?', validation_time)
                .order(taken_at: :asc)
                .first
  end

  # Calculate hours between two times
  def calculate_hours_between(start_time, end_time)
    ((end_time - start_time) / 1.hour).round
  end

  # Add error message for minimum time between doses
  def add_minimum_time_error
    errors.add(:base, "Must wait at least #{prescription.min_hours_between_doses} hours between doses")
  end

  # Format the cycle name for user-friendly display
  def format_cycle_name(cycle)
    return 'day' if cycle.blank?

    case cycle.downcase
    when 'daily'
      'day'
    when 'weekly'
      'week'
    when 'monthly'
      'month'
    else
      cycle
    end
  end
end
