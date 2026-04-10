# frozen_string_literal: true

module Components
  module Medications
    class SupplyStatusCard < Components::Base
      include Phlex::Rails::Helpers::TimeAgoInWords

      attr_reader :medication

      def initialize(medication:)
        @medication = medication
        super()
      end

      def view_template
        Card(class: 'p-8 space-y-6 overflow-hidden relative') do
          Heading(level: 3, size: '4', class: 'font-bold') { t('medications.show.inventory_status') }

          supply_level = medication.supply_level

          div(class: 'space-y-4') do
            div(class: 'flex items-baseline gap-2') do
              stock_count_class = if supply_level.low_stock?
                                    'text-5xl font-black text-on-error-container'
                                  else
                                    'text-5xl font-black text-primary'
                                  end

              span(class: stock_count_class) do
                supply_level.current.to_s
              end
              Text(size: '2', weight: 'bold', class: 'text-muted-foreground') do
                supply_level.current == 1 ? 'unit remaining' : 'units remaining'
              end
            end

            div(class: 'space-y-2') do
              div(class: 'h-2 w-full bg-surface-container-low rounded-full overflow-hidden') do
                div(class: "h-full #{supply_level.low_stock? ? 'bg-error' : 'bg-primary'} rounded-full",
                    style: "width: #{supply_level.percentage}%")
              end
              div(
                class: 'flex justify-between items-center text-[10px] font-black uppercase ' \
                       'tracking-widest text-muted-foreground'
              ) do
                span { t('medications.show.supply_level') }
                span { t('medications.show.reorder_at', threshold: medication.reorder_threshold) }
              end
            end

            if supply_level.low_stock?
              div(class: 'pt-2 space-y-2') do
                Badge(variant: :destructive, class: 'w-full py-2 rounded-xl justify-center text-xs tracking-wide') do
                  t('medications.show.low_stock_alert')
                end

                render_reorder_status_badge if medication.reorder_status.present?
              end
            end

            render_forecast_section
          end
        end
      end

      private

      def render_forecast_section
        if medication.forecast_available?
          div(class: 'pt-4 border-t border-surface-container-low space-y-2') do
            if medication.days_until_low_stock&.positive?
              forecast_item(t('medications.show.forecast.low_in_days', days: medication.days_until_low_stock), :warning)
            end
            if medication.days_until_out_of_stock&.positive?
              forecast_item(t('medications.show.forecast.empty_in_days', days: medication.days_until_out_of_stock),
                            :destructive)
            end
          end
        else
          div(class: 'pt-4 border-t border-surface-container-low') do
            Text(size: '2', class: 'text-muted-foreground italic') { t('medications.show.forecast_unavailable') }
          end
        end
      end

      def forecast_item(message, variant)
        text_class = variant == :destructive ? 'text-on-error-container' : 'text-on-warning-container'

        div(class: 'flex items-center gap-2') do
          render Icons::AlertCircle.new(size: 14, class: text_class)
          Text(size: '2', weight: 'medium', class: text_class) do
            message
          end
        end
      end

      def render_reorder_status_badge
        variant = case medication.reorder_status.to_sym
                  when :ordered then :default
                  when :received then :success
                  else :outline
                  end

        div(class: 'flex flex-col gap-1') do
          Badge(variant: variant, class: 'w-full py-2 rounded-xl justify-center text-xs tracking-wide') do
            t("medications.reorder_statuses.#{medication.reorder_status}")
          end

          timestamp = if medication.reorder_received?
                        medication.reordered_at
                      elsif medication.reorder_ordered?
                        medication.ordered_at
                      end

          if timestamp
            Text(size: '1', class: 'text-center text-muted-foreground font-medium') do
              status_text = t("medications.reorder_statuses.#{medication.reorder_status}")
              time_ago = time_ago_in_words(timestamp)
              "#{status_text} #{time_ago} ago"
            end
          end
        end
      end
    end
  end
end
