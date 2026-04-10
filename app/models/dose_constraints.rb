# frozen_string_literal: true

class DoseConstraints
  attr_reader :max_daily_doses, :min_hours_between_doses

  def initialize(max_daily_doses:, min_hours_between_doses:)
    @max_daily_doses = normalize(max_daily_doses)
    @min_hours_between_doses = normalize(min_hours_between_doses)
  end

  def restrictions?
    daily_limit? || interval_limit?
  end

  def daily_limit?
    max_daily_doses.present?
  end

  def interval_limit?
    min_hours_between_doses.present?
  end

  def would_exceed_daily_limit?(takes:, cycle:, check_time: Time.current)
    return false unless daily_limit?

    range = cycle.range_for(check_time)
    doses_in_cycle = takes.count { |take| range.cover?(take.taken_at) }

    doses_in_cycle >= max_daily_doses
  end

  def would_violate_interval?(takes:, check_time:)
    return false unless interval_limit?

    last_take = takes.select { |take| take.taken_at < check_time }.max_by(&:taken_at)
    return false unless last_take

    hours_since_last = (check_time - last_take.taken_at) / 1.hour
    hours_since_last < min_hours_between_doses
  end

  def satisfied_by?(takes:, check_time:, cycle:)
    return true unless restrictions?

    !would_exceed_daily_limit?(takes: takes, cycle: cycle, check_time: check_time) &&
      !would_violate_interval?(takes: takes, check_time: check_time)
  end

  def next_available_time(takes:, cycle:, now:)
    return nil unless restrictions?
    return now if satisfied_by?(takes: takes, check_time: now, cycle: cycle)

    [
      next_time_from_interval_limit(takes: takes),
      next_time_from_daily_limit(takes: takes, cycle: cycle, now: now)
    ].compact.max
  end

  private

  def normalize(value)
    value.presence
  end

  def next_time_from_interval_limit(takes:)
    return nil unless interval_limit?

    last_take = takes.max_by(&:taken_at)
    return nil unless last_take

    last_take.taken_at + min_hours_between_doses.hours
  end

  def next_time_from_daily_limit(takes:, cycle:, now:)
    return nil unless daily_limit?
    return nil unless would_exceed_daily_limit?(takes: takes, cycle: cycle, check_time: now)

    cycle.next_reset_time(now)
  end
end
