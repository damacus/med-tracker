# frozen_string_literal: true

module SmartInsights
  module Detectors
    class InventoryRisk < Base
      def call
        alert = context.inventory_alerts.first
        return [] unless alert

        [
          insight(
            key: :inventory_risk,
            family: :inventory,
            severity: alert[:low_stock] ? :urgent : :warning,
            title: I18n.t('smart_insights.detectors.inventory_risk.title'),
            summary: summary(alert),
            detail: I18n.t('smart_insights.detectors.inventory_risk.detail', medication_name: alert[:medication_name]),
            metric_label: I18n.t('smart_insights.detectors.inventory_risk.metric_label'),
            metric_value: I18n.t('smart_insights.detectors.inventory_risk.metric_value', count: alert[:days_left])
          )
        ]
      end

      private

      def summary(alert)
        if alert[:days_left] <= 0
          I18n.t('smart_insights.detectors.inventory_risk.summary_zero', medication_name: alert[:medication_name])
        else
          I18n.t(
            'smart_insights.detectors.inventory_risk.summary',
            medication_name: alert[:medication_name],
            count: alert[:days_left]
          )
        end
      end
    end
  end
end
