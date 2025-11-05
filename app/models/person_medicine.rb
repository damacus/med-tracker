# frozen_string_literal: true

# PersonMedicine represents a direct association between a person and a medicine
# without requiring a formal prescription. This is useful for vitamins, supplements,
# and over-the-counter medications.
class PersonMedicine < ApplicationRecord
  belongs_to :person
  belongs_to :medicine
  has_many :medication_takes, dependent: :destroy

  validates :person_id, uniqueness: { scope: :medicine_id }

  def timing_restrictions?
    max_daily_doses.present? || min_hours_between_doses.present?
  end

  def cycle_period
    # For non-prescription medicines, we use daily cycles
    1.day
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
end
