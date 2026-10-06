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
      inputs = inputs_by_source
      sources.flat_map { |source| project_source(source, inputs.fetch(source_key(source))) }
             .sort_by { |row| [row.window_starts_on, *source_key(row.source), row.position] }
    end

    def takes
      @takes ||= eligible_projection_takes.select { |take| take_range.cover?(take.taken_at) }
    end

    private

    def sources
      @sources ||= [Schedule, PersonMedication].flat_map do |model|
        source_scope(model).includes(:person, :medication, :medication_pause_periods).order(:id).to_a
      end
    end

    def source_scope(model)
      scope = model.where(person_id: @people.pluck(:id))
      scope = scope.routine if model == PersonMedication
      foreign_key = model.model_name.singular.foreign_key
      retained_source_scope(scope, foreign_key)
    end

    def retained_source_scope(scope, foreign_key)
      source_ids = scope.select(:id)
      paused_ids = MedicationPausePeriod.where(foreign_key => source_ids).select(foreign_key)
      resolved_ids = MedicationDoseOccurrence.where(foreign_key => source_ids)
                                             .where(window_starts_on: projection_dates, outcome: %w[taken not_taken])
                                             .select(foreign_key)
      scope.current.where(active: true).or(scope.current.where(id: paused_ids)).or(scope.where(id: resolved_ids))
    end

    def inputs_by_source
      takes = eligible_projection_takes.group_by { |take| record_source_key(take) }
      outcomes = outcome_records.group_by { |record| record_source_key(record) }
      sources.to_h do |source|
        key = source_key(source)
        [key, MedicationAdministration::OccurrenceProjection::Inputs.new(
          outcomes: outcomes.fetch(key, []), takes: takes.fetch(key, [])
        )]
      end
    end

    def take_range = @start_date.in_time_zone...(@end_date + 1).in_time_zone

    def eligible_projection_takes
      @eligible_projection_takes ||= begin
        linked_ids = outcome_records.filter_map(&:medication_take_id)
        projection_takes.reject do |take|
          linked_ids.exclude?(take.id) && pause_projection_for(record_source_key(take)).paused_at?(take.taken_at)
        end
      end
    end

    def projection_dates
      first = [@start_date.beginning_of_month, @start_date.beginning_of_week].min
      last = [@end_date.end_of_month, @end_date.end_of_week].max
      first..last
    end

    def source_ids(model)
      sources.grep(model).map(&:id)
    end

    def projection_takes
      @projection_takes ||= begin
        range = projection_dates.begin.in_time_zone...(projection_dates.end + 1).in_time_zone
        MedicationTake.where(schedule_id: source_ids(Schedule), taken_at: take_range)
                      .or(MedicationTake.where(person_medication_id: source_ids(PersonMedication),
                                               taken_at: range)).to_a
      end
    end

    def outcome_records
      @outcome_records ||= begin
        routine = MedicationDoseOccurrence.where(person_medication_id: source_ids(PersonMedication))
        scope = MedicationDoseOccurrence.where(schedule_id: source_ids(Schedule))
                                        .or(routine)
        scope.where(window_starts_on: projection_dates)
             .or(scope.where(medication_take_id: projection_takes.map(&:id))).to_a
      end
    end

    def source_key(source)
      [source.class.name, source.id]
    end

    def record_source_key(record)
      record.schedule_id ? ['Schedule', record.schedule_id] : ['PersonMedication', record.person_medication_id]
    end

    def pause_projection_for(key)
      @pause_projections ||= sources.to_h do |source|
        [source_key(source), MedicationPausePeriods::IntervalProjection.new(periods: source.medication_pause_periods)]
      end
      @pause_projections.fetch(key)
    end

    def project_source(source, inputs)
      pages = (@start_date..@end_date).each_slice(MedicationAdministration::OccurrenceProjection::MAX_DAYS)
      rows = pages.flat_map do |dates|
        MedicationAdministration::OccurrenceProjection.new(
          source: source, start_date: dates.first, end_date: dates.last, now: @now, preloaded: inputs
        ).call
      end
      rows.uniq { |row| [row.window_starts_on, row.position] }
    end
  end
end
