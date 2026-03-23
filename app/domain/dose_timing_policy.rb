# frozen_string_literal: true

class DoseTimingPolicy
  def initialize(takes:, max_daily_doses: nil, min_hours_between_doses: nil, dose_cycle: 'daily')
    @takes = takes
    @max_daily_doses = max_daily_doses
    @min_hours_between_doses = min_hours_between_doses
    @cycle = DoseCycle.new(dose_cycle)
  end

  def restrictions?
    @max_daily_doses.present? || @min_hours_between_doses.present?
  end

  def can_take_at?(check_time = Time.current)
    return true unless restrictions?

    !would_violate_restrictions?(check_time)
  end

  def next_available_time
    return nil unless restrictions?
    return Time.current if can_take_at?

    calculate_next_available_time
  end

  def time_until_next_dose
    return nil if can_take_at?

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

    hours.positive? ? "#{hours}h #{minutes}m" : "#{minutes}m"
  end

  private

  def would_violate_restrictions?(check_time)
    would_exceed_max_doses?(check_time) || would_violate_min_hours?(check_time)
  end

  def would_exceed_max_doses?(check_time)
    return false if @max_daily_doses.blank?

    range = cycle_range(check_time)
    doses_in_cycle = @takes.count { |take| range.cover?(take.taken_at) }

    doses_in_cycle >= @max_daily_doses
  end

  def would_violate_min_hours?(check_time)
    return false if @min_hours_between_doses.blank?

    last_take = @takes.select { |t| t.taken_at < check_time }.max_by(&:taken_at)
    return false if last_take.blank?

    hours_since_last = (check_time - last_take.taken_at) / 1.hour
    hours_since_last < @min_hours_between_doses
  end

  def calculate_next_available_time
    [next_time_from_min_hours, next_time_from_max_doses].compact.max
  end

  def next_time_from_min_hours
    return nil if @min_hours_between_doses.blank?

    last_take = @takes.max_by(&:taken_at)
    return nil unless last_take

    last_take.taken_at + @min_hours_between_doses.hours
  end

  def next_time_from_max_doses
    return nil if @max_daily_doses.blank?
    return nil unless would_exceed_max_doses?(Time.current)

    next_cycle_reset_time(Time.current)
  end

  def cycle_range(time)        = @cycle.range_for(time)
  def next_cycle_reset_time(t) = @cycle.next_reset_time(t)
end
