# frozen_string_literal: true

module TimingRestrictions
  extend ActiveSupport::Concern

  def dose_constraints(date: Time.zone.today)
    @dose_constraints ||= {}

    @dose_constraints[date] ||= DoseConstraints.new(
      max_daily_doses: effective_constraint_value(:max_daily_doses, date),
      min_hours_between_doses: effective_constraint_value(:min_hours_between_doses, date)
    )
  end

  def timing_policy(at: Time.current)
    @timing_policy ||= {}
    date = at.to_date

    cycle = respond_to?(:dose_cycle) ? dose_cycle : 'daily'
    @timing_policy[date] ||= DoseTimingPolicy.new(
      takes: timing_takes(at),
      dose_constraints: dose_constraints(date: date),
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

  def can_take_at?(check_time = Time.current)
    return true unless dose_constraints(date: check_time.to_date).restrictions?

    timing_policy(at: check_time).can_take_at?(check_time)
  end

  def can_take_now?
    can_take_at?
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

  private

  def timing_takes(at)
    return medication_takes.to_a if medication_takes.loaded?

    medication_takes.where(taken_at: (at - 31.days).beginning_of_day..at.end_of_day).to_a
  end

  def effective_constraint_value(attribute, date)
    method_name = "effective_#{attribute}"
    return public_send(method_name, date) if respond_to?(method_name)

    public_send(attribute)
  end
end
