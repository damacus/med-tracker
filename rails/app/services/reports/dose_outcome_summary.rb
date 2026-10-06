module Reports
  class DoseOutcomeSummary
    def initialize(people:, start_date:, end_date:, now: Time.current)
      @now = now
      query = DoseOccurrenceQuery.new(people: people, start_date: start_date, end_date: end_date, now: now)
      partition_rows(query.call)
      @takes_by_date = query.takes.reject { |take| cycle_take?(take) }
                            .group_by { |take| take.taken_at.in_time_zone.to_date }
    end

    def for_date(date)
      rows = @rows_by_date.fetch(date, [])
      { expected: rows.size, actual: @takes_by_date.fetch(date, []).size,
        not_taken: rows.count { |row| row.outcome == 'not_taken' },
        unexplained_missed: rows.count { |row| row.outcome == 'open' && overdue?(row) } }
    end

    def not_taken_outcomes
      (@rows_by_date.values.flatten + @cycle_rows).select { |row| row.outcome == 'not_taken' }
    end

    def cycle_summaries
      grouped = @cycle_rows.group_by { |row| [row.source.id, row.window_starts_on, row.window_ends_on] }
      grouped.values.map { |rows| cycle_summary(rows) }
    end

    private

    def partition_rows(rows)
      relevant = rows.reject { |row| row.outcome == 'open' && !row.expected? }
      @cycle_rows, daily_rows = relevant.partition { |row| row.window_ends_on > row.window_starts_on }
      @rows_by_date = daily_rows.group_by(&:window_starts_on)
    end

    def cycle_summary(rows)
      first = rows.first
      { source: first.source, window_starts_on: first.window_starts_on, window_ends_on: first.window_ends_on,
        expected: rows.size, actual: rows.count { |row| row.outcome == 'taken' },
        not_taken: rows.count { |row| row.outcome == 'not_taken' },
        unexplained_missed: rows.count { |row| row.outcome == 'open' && overdue?(row) } }
    end

    def cycle_take?(take)
      @cycle_rows.any? do |row|
        row.source.id == take.person_medication_id &&
          (row.window_starts_on..row.window_ends_on).cover?(take.taken_at.in_time_zone.to_date)
      end
    end

    def overdue?(row)
      row.scheduled_at ? row.scheduled_at < @now : row.window_ends_on < @now.to_date
    end
  end
end
