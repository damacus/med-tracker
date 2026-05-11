# frozen_string_literal: true

module SmartInsights
  module Detectors
    class PrnUsage < Base
      def call
        return [] unless prn_usage?

        [
          insight(
            key: :prn_usage,
            family: :as_needed,
            severity: :info,
            title: I18n.t('smart_insights.detectors.prn_usage.title'),
            summary: I18n.t('smart_insights.detectors.prn_usage.summary', count: context.prn_takes.count),
            detail: I18n.t('smart_insights.detectors.prn_usage.detail'),
            metric_label: I18n.t('smart_insights.detectors.prn_usage.metric_label'),
            metric_value: I18n.t('smart_insights.detectors.prn_usage.metric_value', count: context.prn_takes.count)
          )
        ]
      end

      private

      def prn_usage?
        context.prn_sources.any? && context.prn_takes.any?
      end
    end
  end
end
