module SmartInsights
  module Detectors
    class MissedRoutineCycles < Base
      def call
        count = longest_streak
        return [] if count < 2

        [insight(key: :missed_routine_cycles, family: :adherence, severity: :warning,
                 title: translate('title'), summary: translate('summary', count: count),
                 detail: translate('detail'), metric_label: translate('metric_label'),
                 metric_value: translate('metric_value', count: count))]
      end

      private

      def longest_streak
        completed = context.cycle_summaries.select { |cycle| cycle[:window_ends_on] < Date.current }
        completed.group_by { |cycle| cycle[:source].id }.values.map { |cycles| source_streak(cycles) }.max || 0
      end

      def source_streak(cycles)
        previous_end = nil
        current = longest = 0
        cycles.sort_by { |cycle| cycle[:window_starts_on] }.each do |cycle|
          current = 0 if previous_end != cycle[:window_starts_on] - 1
          current = cycle[:unexplained_missed].positive? ? current + 1 : 0
          longest = [longest, current].max
          previous_end = cycle[:window_ends_on]
        end
        longest
      end

      def translate(key, **)
        I18n.t("smart_insights.detectors.missed_routine_cycles.#{key}", **)
      end
    end
  end
end
