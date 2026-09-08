module Reports
  class DoseOutcomeSummary
    def initialize(people:, start_date:, end_date:, now: Time.current)
      @now = now
      query = DoseOccurrenceQuery.new(people: people, start_date: start_date, end_date: end_date, now: now)
      rows = query.call.reject { |row| row.outcome == 'open' && !row.expected? }
      @rows_by_date = rows.group_by(&:window_starts_on)
      @takes_by_date = query.takes.group_by { |take| take.taken_at.in_time_zone.to_date }
    end

    def for_date(date)
      rows = @rows_by_date.fetch(date, [])
      { expected: rows.size, actual: @takes_by_date.fetch(date, []).size,
        not_taken: rows.count { |row| row.outcome == 'not_taken' },
        unexplained_missed: rows.count { |row| row.outcome == 'open' && overdue?(row) } }
    end

    private

    def overdue?(row)
      row.scheduled_at ? row.scheduled_at < @now : row.window_starts_on < @now.to_date
    end
  end
end
