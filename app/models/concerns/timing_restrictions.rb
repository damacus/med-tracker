# frozen_string_literal: true

# Shared concern for timing restrictions on medications.
# Delegates pure timing logic to DoseTimingPolicy (a domain policy object),
# keeping stock-awareness (can_administer?, administration_blocked_reason)
# at the model level where the medication association lives.
module TimingRestrictions
  extend ActiveSupport::Concern

  def timing_policy
    cycle = respond_to?(:dose_cycle) ? dose_cycle : 'daily'
    takes = if medication_takes.loaded?
              medication_takes.to_a
            else
              medication_takes.where(taken_at: 30.days.ago..Time.current.end_of_day).to_a
            end
    DoseTimingPolicy.new(
      takes: takes,
      max_daily_doses: max_daily_doses,
      min_hours_between_doses: min_hours_between_doses,
      dose_cycle: cycle
    )
  end

  def timing_restrictions? = timing_policy.restrictions?
  def can_take_now? = timing_policy.can_take_at?
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
