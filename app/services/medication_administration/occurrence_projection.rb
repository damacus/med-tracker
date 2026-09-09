module MedicationAdministration
  class OccurrenceProjection
    MAX_DAYS = 31
    Inputs = Data.define(:outcomes, :takes)
    Occurrence = Data.define(
      :key, :source, :window_starts_on, :position, :scheduled_at, :record, :now, :expected, :legacy_take
    ) do
      def outcome = legacy_take ? 'taken' : record&.outcome || 'open'

      def window_ends_on
        return record.window_ends_on if record&.window_ends_on
        return window_starts_on if source.is_a?(Schedule)

        DoseCycle.new(source.dose_cycle).range_for(window_starts_on.in_time_zone).end.to_date
      end

      def due?
        (scheduled_at || window_starts_on.in_time_zone) <= now &&
          (source.is_a?(Schedule) || source.created_at <= now)
      end

      def expected? = expected
    end

    def self.verifier = Rails.application.message_verifier('medication_dose_occurrence')

    def self.decode(key)
      return if key.to_s.bytesize > 1024

      verifier.verified(key.to_s)
    end

    def initialize(source:, start_date:, end_date:, now: Time.current, preloaded: nil)
      @source = source
      @start_date = start_date
      @end_date = end_date
      @now = now
      @preloaded = preloaded
      validate_range!
      @source_type = MedicationDoseSource.new(source).type
      @start_date = window_start(start_date)
      @end_date = window_finish(end_date) - 1
    end

    def call
      rows = merged_rows.sort_by { |row| [row.window_starts_on, row.position] }
      allocate_legacy_takes(rows)
    end

    private

    attr_reader :source, :start_date, :end_date, :now, :source_type

    def routine_source? = source_type == 'person_medication'

    def window_start(date)
      routine_source? ? DoseCycle.new(source.dose_cycle).range_for(date.in_time_zone).begin.to_date : date
    end

    def window_finish(date)
      return date + 1 unless routine_source?

      DoseCycle.new(source.dose_cycle).range_for(date.in_time_zone).end.to_date + 1
    end

    def merged_rows
      rows = derived_rows.index_by { |row| [row.window_starts_on, row.position] }
      persisted_outcomes.each do |record|
        identity = [record.window_starts_on, record.position]
        rows[identity] = persisted_row(record).with(expected: rows.key?(identity))
      end
      rows.values
    end

    def validate_range!
      return if start_date.is_a?(Date) && end_date.is_a?(Date) &&
                end_date >= start_date && (end_date - start_date) < MAX_DAYS

      raise ArgumentError, 'A valid date range of at most 31 days is required'
    end

    def derived_rows
      return [] if as_needed?
      return routine_rows if routine_source?
      return [] if !source.active && pause_periods.empty?

      (start_date..end_date).flat_map { |date| eligible_date?(date) ? rows_on(date) : [] }
    end

    def routine_rows
      return [] unless routine_history_available?

      (start_date..end_date).map { |date| window_start(date) }.uniq.flat_map do |date|
        eligible_routine_window?(date) ? untimed_rows(date) : []
      end
    end

    def routine_history_available?
      source.active || source.retired_at.present? || pause_periods.any?
    end

    def eligible_routine_window?(date)
      window_finish(date) > source.created_at.to_date &&
        (source.retired_at.nil? || date < source.retired_at.to_date)
    end

    def eligible_date?(date)
      source.applies_on?(date) && (source.retired_at.nil? || date < source.retired_at.to_date)
    end

    def persisted_row(record)
      occurrence(record.window_starts_on, record.position, record.scheduled_at, record: record)
    end

    def as_needed?
      return source.as_needed? if routine_source?

      source.schedule_type_prn? || source.frequency.to_s.casecmp('as needed').zero? ||
        source.schedule_config.to_h['as_needed'] == true
    end

    def rows_on(date)
      times = MedicationPausePeriods::IntervalProjection.occurrences_on(
        date: date, times: source.schedule_config.to_h['times']
      )
      return untimed_rows(date) if times.empty?

      times.each_with_index.filter_map do |time, index|
        occurrence(date, index + 1, time) unless pause_projection.paused_at?(time)
      end
    end

    def untimed_rows(date)
      return [] if fully_paused?(date)

      count = routine_source? ? source.max_daily_doses.presence || 1 : source.expected_doses_on(date)
      Array.new(count) { |index| occurrence(date, index + 1, nil) }
    end

    def fully_paused?(date)
      cursor, finish = active_window_bounds(date)
      sorted_pause_periods.each do |period|
        starts_at = period.started_at || period.created_at
        next if starts_at > cursor

        cursor = [cursor, period.ended_at || finish].max
        return true if cursor >= finish
      end
      false
    end

    def active_window_bounds(date)
      first = date.in_time_zone
      finish = window_finish(date).in_time_zone
      return [first, finish] unless routine_source?

      [[first, source.created_at].max, [finish, source.retired_at].compact.min]
    end

    def sorted_pause_periods
      pause_periods.sort_by { |period| period.started_at || period.created_at }
    end

    def pause_periods = @pause_periods ||= source.medication_pause_periods.to_a

    def pause_projection
      @pause_projection ||= MedicationPausePeriods::IntervalProjection.new(periods: pause_periods)
    end

    def persisted_outcomes
      if @preloaded
        return @preloaded.outcomes.select do |record|
          matches_source?(record) && (start_date..end_date).cover?(record.window_starts_on)
        end
      end

      @persisted_outcomes ||= source.medication_dose_occurrences.where(window_starts_on: start_date..end_date).to_a
    end

    def allocate_legacy_takes(rows)
      takes_by_date = legacy_takes.group_by { |take| window_start(take.taken_at.in_time_zone.to_date) }
      rows.map do |row|
        next row unless row.expected? && row.outcome == 'open'

        row.with(legacy_take: takes_by_date[row.window_starts_on]&.shift)
      end
    end

    def legacy_takes
      @preloaded ? preloaded_legacy_takes : queried_legacy_takes
    end

    def queried_legacy_takes
      linked_ids = source.medication_dose_occurrences.where.not(medication_take_id: nil).select(:medication_take_id)
      source.medication_takes.where(taken_at: start_date.in_time_zone...(end_date + 1).in_time_zone)
            .where.not(id: linked_ids).order(:taken_at, :id)
    end

    def preloaded_legacy_takes
      linked_ids = @preloaded.outcomes.filter_map(&:medication_take_id)
      takes = @preloaded.takes.select do |take|
        preloaded_take_in_range?(take) && linked_ids.exclude?(take.id)
      end
      takes.sort_by { |take| [take.taken_at, take.id] }
    end

    def preloaded_take_in_range?(take)
      matches_source?(take) && (start_date..end_date).cover?(take.taken_at.in_time_zone.to_date)
    end

    def matches_source?(record)
      record.public_send("#{source_type}_id") == source.id
    end

    def occurrence(date, position, scheduled_at, record: nil)
      Occurrence.new(
        key: self.class.verifier.generate([source_type, source.portable_id, date.iso8601, position]),
        source: source, window_starts_on: date, position: position, scheduled_at: scheduled_at,
        record: record, now: now, expected: true, legacy_take: nil
      )
    end
  end
end
