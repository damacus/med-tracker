module FamilyDashboard
  module RoutineDoseProgress
    private

    def current_not_taken_outcomes(source)
      outcomes = outcome_query.for_source(source)
      return outcomes unless source.is_a?(Schedule)

      positions = configured_times_for(source).size.nonzero? || expected_routine_doses_for(source)
      outcomes.select { |outcome| outcome.position <= positions }
    end

    def dose_progress_for(takes, limit)
      { daily_dose_count: takes.size, daily_dose_limit: limit, today_takes: takes.sort_by(&:taken_at) }
    end

    def expected_routine_doses_for(source)
      source.is_a?(Schedule) ? expected_schedule_doses_for(source) : source.max_daily_doses.presence || 1
    end

    def expected_schedule_doses_for(schedule)
      return 0 unless schedule.applies_on?(Date.current)

      configured_doses = configured_schedule_doses_for(schedule)
      return configured_doses unless configured_doses.nil?

      expected = schedule.expected_doses_on(Date.current)
      return expected unless expected == 1 && schedule.effective_max_daily_doses.blank?
      return expected if schedule.effective_min_hours_between_doses.blank?

      (24 / schedule.effective_min_hours_between_doses.to_f).ceil
    end

    def configured_schedule_doses_for(schedule)
      return if configured_times_for(schedule).blank?

      active_configured_occurrences_for(schedule).size
    end

    def taken_count_for_cycle(source, now)
      cycle = source_cycle(source)
      source.medication_takes.count { |take| cycle.range_for(now).cover?(take.taken_at) }
    end
  end
end
