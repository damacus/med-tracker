# frozen_string_literal: true

module TimingRestrictions
  extend ActiveSupport::Concern

  def dose_constraints
    return @dose_constraints if defined?(@dose_constraints)

    @dose_constraints = DoseConstraints.new(
      max_daily_doses: max_daily_doses,
      min_hours_between_doses: min_hours_between_doses
    )
  end

  def timing_policy
    return @timing_policy if defined?(@timing_policy)

    cycle = respond_to?(:dose_cycle) ? dose_cycle : 'daily'
    takes = if medication_takes.loaded?
              medication_takes.to_a
            else
              medication_takes.where(taken_at: 31.days.ago.beginning_of_day..Time.current.end_of_day).to_a
            end
    @timing_policy = DoseTimingPolicy.new(
      takes: takes,
      dose_constraints: dose_constraints,
      dose_cycle: cycle
    )
  end

  def reload(*)
    remove_instance_variable(:@timing_policy) if defined?(@timing_policy)
    remove_instance_variable(:@dose_constraints) if defined?(@dose_constraints)
    super
  end

  delegate :restrictions?, to: :dose_constraints, prefix: false

  alias timing_restrictions? restrictions?

  def can_take_now?
    return true unless timing_restrictions?

    timing_policy.can_take_at?
  end
  delegate :next_available_time, :time_until_next_dose, :countdown_display, to: :timing_policy

  def can_administer?
    return false if medication.out_of_stock?

    can_take_now?
  end

  def administration_blocked_reason
    return :out_of_stock if medication.out_of_stock?
    return :cooldown unless can_take_now?

    nil
  end
end
