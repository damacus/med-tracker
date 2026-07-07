# frozen_string_literal: true

module SmartInsights
  module Detectors
    class TimingConsistency < Base
      WINDOW_MINUTES = 90

      def call
        return [] unless enough_timing_evidence?
        return [] if on_time_ratio < 0.8

        [timing_insight]
      end

      private

      def timing_insight
        insight(
          key: :timing_consistency,
          family: :timing,
          severity: :positive,
          title: I18n.t('smart_insights.detectors.timing_consistency.title'),
          summary: I18n.t(
            'smart_insights.detectors.timing_consistency.summary',
            percentage: (on_time_ratio * 100).round
          ),
          detail: I18n.t('smart_insights.detectors.timing_consistency.detail'),
          metric_label: I18n.t('smart_insights.detectors.timing_consistency.metric_label'),
          metric_value: I18n.t('smart_insights.detectors.timing_consistency.metric_value', count: on_time_count)
        )
      end

      def enough_timing_evidence?
        expected_occurrences.size >= IndexQuery::MINIMUM_EVENTS
      end

      def on_time_ratio
        on_time_count.to_f / expected_occurrences.size
      end

      def on_time_count
        @on_time_count ||= begin
          remaining_takes_by_schedule_id = takes_with_configured_times.group_by(&:schedule_id)

          expected_occurrences.count do |occurrence|
            remaining_takes = remaining_takes_by_schedule_id.fetch(occurrence[:schedule].id, [])
            matching_take = remaining_takes.find { |take| on_time?(take, occurrence) }
            remaining_takes.delete(matching_take) if matching_take
          end
        end
      end

      def takes_with_configured_times
        @takes_with_configured_times ||= context.takes.select { |take| configured_times(take.schedule).any? }
      end

      def expected_occurrences
        @expected_occurrences ||= context.schedules.flat_map do |schedule|
          configured_times(schedule).flat_map do |time|
            (context.start_date..context.end_date).filter_map do |date|
              next unless schedule.expected_doses_on(date).positive?

              { schedule: schedule, expected_at: expected_time(date, time) }
            end
          end
        end
      end

      def on_time?(take, occurrence)
        take.schedule_id == occurrence[:schedule].id &&
          (take.taken_at - occurrence[:expected_at]).abs <= WINDOW_MINUTES.minutes
      end

      def configured_times(schedule)
        Array(schedule&.schedule_config.to_h['times']).compact_blank
      end

      def expected_time(date, time)
        hour, minute = time.split(':').map(&:to_i)
        date.in_time_zone.change(hour: hour, min: minute)
      end
    end
  end
end
