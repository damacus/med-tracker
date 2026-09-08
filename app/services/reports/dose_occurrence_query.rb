module Reports
  class DoseOccurrenceQuery
    def initialize(people:, start_date:, end_date:, now: Time.current)
      @people = people
      @start_date = start_date
      @end_date = end_date
      @now = now
      DateRange.new(start_date: start_date, end_date: end_date).validate!
    end

    def call
      inputs = inputs_by_schedule
      schedules.flat_map { |source| project_source(source, inputs.fetch(source.id)) }
               .sort_by { |row| [row.window_starts_on, row.source.id, row.position] }
    end

    def takes
      @takes ||= begin
        linked_ids = outcome_records.filter_map(&:medication_take_id)
        medication_takes.reject do |take|
          linked_ids.exclude?(take.id) && pause_projection_for(take.schedule_id).paused_at?(take.taken_at)
        end
      end
    end

    private

    def schedules
      @schedules ||= source_scope.includes(:person, :medication, :medication_pause_periods).order(:id).to_a
    end

    def source_scope
      scope = Schedule.where(person_id: @people.pluck(:id))
      source_ids = scope.select(:id)
      paused_ids = MedicationPausePeriod.where(schedule_id: source_ids).select(:schedule_id)
      resolved_ids = MedicationDoseOccurrence.where(
        schedule_id: source_ids, window_starts_on: @start_date..@end_date, outcome: %w[taken not_taken]
      ).select(:schedule_id)
      scope.current.where(active: true).or(scope.current.where(id: paused_ids)).or(scope.where(id: resolved_ids))
    end

    def inputs_by_schedule
      takes = self.takes.group_by(&:schedule_id)
      outcomes = outcome_records.group_by(&:schedule_id)
      schedules.to_h do |source|
        [source.id, MedicationAdministration::OccurrenceProjection::Inputs.new(
          outcomes: outcomes.fetch(source.id, []), takes: takes.fetch(source.id, [])
        )]
      end
    end

    def medication_takes
      @medication_takes ||= begin
        range = @start_date.in_time_zone...(@end_date + 1).in_time_zone
        MedicationTake.where(schedule_id: schedules.map(&:id), taken_at: range).to_a
      end
    end

    def outcome_records
      @outcome_records ||= begin
        scope = MedicationDoseOccurrence.where(schedule_id: schedules.map(&:id))
        scope.where(window_starts_on: @start_date..@end_date)
             .or(scope.where(medication_take_id: medication_takes.map(&:id))).to_a
      end
    end

    def pause_projection_for(schedule_id)
      @pause_projections ||= schedules.to_h do |source|
        [source.id, MedicationPausePeriods::IntervalProjection.new(periods: source.medication_pause_periods)]
      end
      @pause_projections.fetch(schedule_id)
    end

    def project_source(source, inputs)
      (@start_date..@end_date).each_slice(MedicationAdministration::OccurrenceProjection::MAX_DAYS).flat_map do |dates|
        MedicationAdministration::OccurrenceProjection.new(
          source: source, start_date: dates.first, end_date: dates.last, now: @now, preloaded: inputs
        ).call
      end
    end
  end
end
