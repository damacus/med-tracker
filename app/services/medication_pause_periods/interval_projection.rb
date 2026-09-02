# frozen_string_literal: true

module MedicationPausePeriods
  class IntervalProjection
    class << self
      def occurrences_on(date:, times:)
        Array(times).compact_blank.filter_map { |raw_time| occurrence_on(date, raw_time) }
      end

      private

      def occurrence_on(date, raw_time)
        hour, minute = raw_time.to_s.split(':', 3).first(2)
        return unless valid_clock_time?(hour, minute)

        Time.zone.local(date.year, date.month, date.day, hour.to_i, minute.to_i)
      end

      def valid_clock_time?(hour, minute)
        hour&.match?(/\A\d+\z/) && minute&.match?(/\A\d+\z/) &&
          (0..23).cover?(hour.to_i) && (0..59).cover?(minute.to_i)
      end
    end

    def initialize(periods:)
      @periods = periods
    end

    def active_occurrences(occurrences)
      occurrences.reject { |occurrence| paused_at?(occurrence) }
    end

    def paused_at?(occurrence)
      periods.any? { |period| covers?(period, occurrence) }
    end

    private

    attr_reader :periods

    def covers?(period, occurrence)
      started_at = period.started_at || period.created_at
      started_at.present? && started_at <= occurrence && (period.ended_at.nil? || occurrence < period.ended_at)
    end
  end
end
