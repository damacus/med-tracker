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

          div(class: 'space-y-4') do
            div(class: 'flex items-baseline gap-2') do
              span(class: presenter.stock_count_class) do
                presenter.supply_level.current.to_s
              end
              Text(size: '2', weight: 'bold', class: 'text-muted-foreground') do
                presenter.remaining_units_label
              end
            end

            div(class: 'space-y-2') do
              div(class: 'h-2 w-full bg-card rounded-full overflow-hidden') do
                div(class: "h-full #{presenter.supply_bar_class} rounded-full",
                    style: "width: #{presenter.supply_level.percentage}%")
              end
              div(
                class: 'flex justify-between items-center text-[10px] font-black uppercase ' \
                       'tracking-widest text-muted-foreground'
              ) do
                span { t('medications.show.supply_level') }
                span { t('medications.show.reorder_at', threshold: medication.reorder_threshold) }
              end
            end

            if presenter.supply_level.low_stock?
              div(class: 'pt-2 space-y-2') do
                Badge(variant: :destructive, class: 'w-full py-2 rounded-xl justify-center text-xs tracking-wide') do
                  t('medications.show.low_stock_alert')
                end

                render_reorder_status_badge if presenter.reorder_status_badge?
              end
            end

            render_forecast_section
          end
        end
      end

      private

      def presenter
        @presenter ||= ::Medications::SupplyStatusPresenter.new(medication: medication)
      end

      def render_forecast_section
        if presenter.forecast_items.any?
          div(class: 'pt-4 border-t border-surface-container-low space-y-2') do
            presenter.forecast_items.each do |item|
              forecast_item(item[:message], item[:variant])
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
        div(class: 'flex flex-col gap-1') do
          Badge(
            variant: presenter.reorder_status_variant,
            class: 'w-full py-2 rounded-xl justify-center text-xs tracking-wide'
          ) do
            presenter.reorder_status_label
          end

          if presenter.reorder_status_timestamp
            Text(size: '1', class: 'text-center text-muted-foreground font-medium') do
              status_text = presenter.reorder_status_label
              time_ago = time_ago_in_words(presenter.reorder_status_timestamp)
              "#{status_text} #{time_ago} ago"
            end
          end
        end
      end
    end
  end
end
