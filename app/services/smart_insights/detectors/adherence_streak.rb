# frozen_string_literal: true

module SmartInsights
  module Detectors
    class AdherenceStreak < Base
      def call
        streak = current_streak
        return [] if streak < 3

        [
          insight(
            key: :adherence_streak,
            family: :adherence,
            severity: :positive,
            title: I18n.t('smart_insights.detectors.adherence_streak.title'),
            summary: I18n.t('smart_insights.detectors.adherence_streak.summary', count: streak),
            detail: I18n.t('smart_insights.detectors.adherence_streak.detail'),
            metric_label: I18n.t('smart_insights.detectors.adherence_streak.metric_label'),
            metric_value: I18n.t('smart_insights.detectors.adherence_streak.metric_value', count: streak)
          )
        ]
      end

      private

      def current_streak
        context.daily_data.reverse.take_while do |day|
          day[:expected].positive? && day[:actual] >= day[:expected]
        end.count
      end
    end
  end
end
