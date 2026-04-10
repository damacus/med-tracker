# frozen_string_literal: true

class DoseTimingPolicy
  def initialize(takes:, dose_constraints:, dose_cycle: 'daily')
    @takes = takes
    @dose_constraints = dose_constraints
    @cycle = DoseCycle.new(dose_cycle)
  end

  delegate :restrictions?, to: :@dose_constraints

  def can_take_at?(check_time = Time.current)
    return true unless restrictions?

    @dose_constraints.satisfied_by?(takes: @takes, check_time: check_time, cycle: @cycle)
  end

  def next_available_time
    return nil unless restrictions?
    return Time.current if can_take_at?

    @dose_constraints.next_available_time(takes: @takes, cycle: @cycle, now: Time.current)
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
end
