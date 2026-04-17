# frozen_string_literal: true

module Medications
  class SupplyStatusPresenter
    attr_reader :medication

    delegate :supply_level, to: :medication

    def initialize(medication:)
      @medication = medication
    end

    def status_variant
      return :default if medication.reorder_ordered?
      return :success if medication.reorder_received?

      case supply_level.status
      when :out_of_stock then :destructive
      when :low_stock then :warning
      else :success
      end
    end

    def status_label
      return I18n.t('medications.reorder_statuses.ordered') if medication.reorder_ordered?
      return I18n.t('medications.reorder_statuses.received') if medication.reorder_received?

      case supply_level.status
      when :out_of_stock then I18n.t('dashboard.statuses.out_of_stock')
      when :low_stock then I18n.t('medications.show.low_stock_alert')
      else I18n.t('medications.index.in_stock', default: 'In Stock')
      end
    end

    def stock_count_class
      if supply_level.low_stock?
        'text-5xl font-black text-on-error-container'
      else
        'text-5xl font-black text-primary'
      end
    end

    def supply_bar_class
      supply_level.low_stock? ? 'bg-error' : 'bg-primary'
    end

    def list_supply_bar_class
      supply_level.low_stock? ? 'bg-destructive' : 'bg-primary'
    end

    def remaining_units_label
      supply_level.current == 1 ? 'unit remaining' : 'units remaining'
    end

    def inventory_units_label
      ActionController::Base.helpers.pluralize(supply_level.current, 'unit')
    end

    def forecast_items
      return [] unless medication.forecast_available?

      [low_stock_forecast, out_of_stock_forecast].compact
    end

    def reorder_status_badge?
      supply_level.low_stock? && medication.reorder_status.present?
    end

    def reorder_status_variant
      case medication.reorder_status&.to_sym
      when :ordered then :default
      when :received then :success
      else :outline
      end
    end

    def reorder_status_label
      I18n.t("medications.reorder_statuses.#{medication.reorder_status}")
    end

    def reorder_status_timestamp
      return medication.reordered_at if medication.reorder_received?
      return medication.ordered_at if medication.reorder_ordered?

      nil
    end

    private

    def low_stock_forecast
      return unless medication.days_until_low_stock&.positive?

      {
        message: I18n.t('medications.show.forecast.low_in_days', days: medication.days_until_low_stock),
        variant: :warning
      }
    end

    def out_of_stock_forecast
      return unless medication.days_until_out_of_stock&.positive?

      {
        message: I18n.t(
          'medications.show.forecast.empty_in_days',
          days: medication.days_until_out_of_stock
        ),
        variant: :destructive
      }
    end
  end
end
