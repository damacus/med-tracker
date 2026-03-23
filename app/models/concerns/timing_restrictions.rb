# frozen_string_literal: true

module TimingRestrictions
  extend ActiveSupport::Concern

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
      max_daily_doses: max_daily_doses,
      min_hours_between_doses: min_hours_between_doses,
      dose_cycle: cycle
    )
  end

  def reload(*)
    @timing_policy = nil
    super
  end

  def timing_restrictions?
    max_daily_doses.present? || min_hours_between_doses.present?
  end

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
