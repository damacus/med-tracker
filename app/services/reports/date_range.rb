# frozen_string_literal: true

module Reports
  class DateRange
    class RangeTooLarge < ArgumentError; end

    MAX_RANGE_DAYS = 180
    DEFAULT_RANGE_DAYS = 6

    attr_reader :start_date, :end_date

    def self.parse(start_date:, end_date:, default_end_date: Time.zone.today, default_range_days: DEFAULT_RANGE_DAYS)
      end_on = end_date.present? ? Date.parse(end_date.to_s) : default_end_date
      start_on = start_date.present? ? Date.parse(start_date.to_s) : end_on - default_range_days
      new(start_date: start_on, end_date: end_on).tap(&:validate!)
    end

    def initialize(start_date:, end_date:)
      @start_date = start_date
      @end_date = end_date
    end

    def validate!
      raise ArgumentError, 'end_date must be on or after start_date' if end_date < start_date
      raise RangeTooLarge if (end_date - start_date).to_i > MAX_RANGE_DAYS

      self
    end

    def to_h
      { start_date: start_date, end_date: end_date }
    end
  end
end
