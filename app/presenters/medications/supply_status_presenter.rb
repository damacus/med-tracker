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
      list_stock_alert? ? 'bg-destructive' : 'bg-primary'
    end

    def list_inventory_text_class
      list_stock_alert? ? 'text-destructive' : 'text-primary'
    end

    def remaining_units_label
      return 'ml remaining' if volume_stock?

      supply_level.current == 1 ? 'unit remaining' : 'units remaining'
    end

    def inventory_units_label
      return "#{formatted_supply_current} ml" if volume_stock?

      formatted_supply_current == '1' ? '1 unit' : "#{formatted_supply_current} units"
    end

    def formatted_supply_current
      MedicationStockQuantityFormatter.format(supply_level.current)
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

    def volume_stock?
      MedicationStockConsumption.volume_unit?(medication.dosage_unit)
    end

    def list_stock_alert?
      return false unless supply_level.tracked?
      return scheduled_stock_alert? if scheduled_inventory?
      return as_needed_stock_alert? if as_needed_inventory?

      supply_level.low_stock?
    end

    def scheduled_stock_alert?
      medication.days_until_out_of_stock.present? && medication.days_until_out_of_stock < 5
    end

    def as_needed_stock_alert?
      as_needed_doses_left < 10
    end

    def as_needed_doses_left
      BigDecimal(supply_level.current.to_s) / as_needed_dose_quantity
    end

    def as_needed_dose_quantity
      quantities = as_needed_sources.map { |source| dose_quantity_for(source) }.select(&:positive?)

      quantities.max || BigDecimal('1')
    end

    def dose_quantity_for(source)
      MedicationStockConsumption.quantity_for(
        dose_amount: dose_amount_for(source),
        dose_unit: dose_unit_for(source)
      )
    end

    def dose_amount_for(source)
      return source.effective_dose_amount if source.respond_to?(:effective_dose_amount)

      source.default_dose_amount
    end

    def dose_unit_for(source)
      return source.effective_dose_unit if source.respond_to?(:effective_dose_unit)

      source.dose_unit
    end

    def scheduled_inventory?
      scheduled_sources.any?
    end

    def as_needed_inventory?
      as_needed_sources.any?
    end

    def scheduled_sources
      @scheduled_sources ||= medication.schedules.select do |schedule|
        schedule.active? && !as_needed_schedule?(schedule)
      end
    end

    def as_needed_sources
      @as_needed_sources ||= medication.person_medications.select do |person_medication|
        person_medication.active? && person_medication.as_needed?
      end + as_needed_schedules
    end

    def as_needed_schedules
      medication.schedules.select do |schedule|
        schedule.active? && as_needed_schedule?(schedule)
      end
    end

    def as_needed_schedule?(schedule)
      schedule.schedule_type_prn? ||
        schedule.schedule_config.to_h['as_needed'] == true ||
        schedule.frequency.to_s.casecmp('as needed').zero?
    end

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
