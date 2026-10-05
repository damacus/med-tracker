# frozen_string_literal: true

module Reports
  class GpDateRange < DateRange
    MAX_RANGE_DAYS = 366

    def self.parse(start_date:, end_date:, default_end_date: Time.zone.today)
      super(
        start_date:,
        end_date:,
        default_end_date:,
        default_range_days: 1.year
      )
    end

    def validate!
      raise EndBeforeStart, 'end_date must be on or after start_date' if end_date < start_date
      raise RangeTooLarge if (end_date - start_date).to_i > MAX_RANGE_DAYS

      self
    end
  end
end
