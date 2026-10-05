# frozen_string_literal: true

module SmartInsights
  module Detectors
    class ScheduleHygiene < Base
      def call
        schedule = schedules_without_times.first
        return [] unless schedule

        [schedule_hygiene_insight(schedule)]
      end

      private

      def schedule_hygiene_insight(schedule)
        insight(
          key: :schedule_hygiene,
          family: :schedule,
          severity: :info,
          title: I18n.t('smart_insights.detectors.schedule_hygiene.title'),
          summary: I18n.t('smart_insights.detectors.schedule_hygiene.summary',
                          medication_name: schedule.medication_name),
          detail: I18n.t('smart_insights.detectors.schedule_hygiene.detail'),
          metric_label: I18n.t('smart_insights.detectors.schedule_hygiene.metric_label'),
          metric_value: I18n.t('smart_insights.detectors.schedule_hygiene.metric_value')
        )
      end

      def schedules_without_times
        context.active_schedules.select do |schedule|
          schedule.schedule_type_multiple_daily? && Array(schedule.schedule_config.to_h['times']).compact_blank.empty?
        end
      end
    end
  end
end
