module Reports
  class DoseOutcomeSummary
    def initialize(schedules:, takes_by_date:, start_date:, end_date:)
      @schedules = schedules
      @takes_by_date = takes_by_date
      @date_range = start_date..end_date
    end

    def for_date(date, expected_by_schedule:)
      taken = @takes_by_date.fetch(date, []).group_by(&:schedule_id).transform_values(&:size)
      unexplained = expected_by_schedule.sum do |id, expected|
        [expected - taken.fetch(id, 0) - not_taken_counts.fetch([date, id], 0), 0].max
      end
      { not_taken: not_taken_counts.sum { |(day, _id), count| day == date ? count : 0 },
        unexplained_missed: unexplained }
    end

    private

    def not_taken_counts
      @not_taken_counts ||= MedicationDoseOccurrence.where(
        schedule_id: @schedules.map(&:id), window_starts_on: @date_range, outcome: 'not_taken'
      ).group(:window_starts_on, :schedule_id).count
    end
  end
end
