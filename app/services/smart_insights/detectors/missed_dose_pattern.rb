# frozen_string_literal: true

module SmartInsights
  module Detectors
    class MissedDosePattern < Base
      def call
        streak = longest_missed_streak
        return [] if streak < 2

        [
          insight(
            key: :missed_dose_pattern,
            family: :adherence,
            severity: :warning,
            title: I18n.t('smart_insights.detectors.missed_dose_pattern.title'),
            summary: I18n.t('smart_insights.detectors.missed_dose_pattern.summary', count: streak),
            detail: I18n.t('smart_insights.detectors.missed_dose_pattern.detail'),
            metric_label: I18n.t('smart_insights.detectors.missed_dose_pattern.metric_label'),
            metric_value: I18n.t('smart_insights.detectors.missed_dose_pattern.metric_value', count: streak)
          )
        ]
      end

      private

      def longest_missed_streak
        context.daily_data.each_with_object([0, 0]) do |day, streaks|
          current, longest = streaks
          current = missed_day?(day) ? current + 1 : 0
          streaks[0] = current
          streaks[1] = [longest, current].max
        end.last
      end

      def missed_day?(day)
        day[:expected].positive? && day[:actual] < day[:expected]
      end
    end
  end
end
