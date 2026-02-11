# frozen_string_literal: true

# Shared concern for timing restrictions on medications
# Handles logic for determining when a medication can be taken based on:
# - Maximum daily doses
# - Minimum hours between doses
module TimingRestrictions
  extend ActiveSupport::Concern

  def timing_restrictions?
    max_daily_doses.present? || min_hours_between_doses.present?
  end

  def can_take_now?
    return true unless timing_restrictions?

    !would_violate_restrictions?(Time.current)
  end

  def can_administer?
    return false if medicine.out_of_stock?

    can_take_now?
  end

  def administration_blocked_reason
    return :out_of_stock if medicine.out_of_stock?
    return :cooldown unless can_take_now?

    nil
  end

  def next_available_time
    return nil unless timing_restrictions?
    return Time.current if can_take_now?

    calculate_next_available_time
  end

  def time_until_next_dose
    return nil if can_take_now?

    next_time = next_available_time
    return nil unless next_time

    (next_time - Time.current).to_i
  end

  def countdown_display
    seconds = time_until_next_dose
    return nil unless seconds

    return 'less than 1 minute' if seconds < 60

    hours = seconds / 3600
    minutes = (seconds % 3600) / 60

    if hours.positive?
      "#{hours}h #{minutes}m"
    else
      "#{minutes}m"
    end
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
end
