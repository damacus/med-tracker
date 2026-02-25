# frozen_string_literal: true

module Components
  module Medicines
    class RefillModal < Components::Base
      include Phlex::Rails::Helpers::FormWith

      attr_reader :medicine, :button_variant, :button_class, :quantity, :restock_date, :icon_only, :button_label

      # rubocop:disable Metrics/ParameterLists
      def initialize(medicine:, button_variant: :outline, button_class: '',
                     quantity: nil, restock_date: nil, icon_only: false, button_label: nil)
        @medicine = medicine
        @button_variant = button_variant
        @button_class = button_class
        @quantity = quantity
        @restock_date = restock_date
        @icon_only = icon_only
        @button_label = button_label
        super()
      end
      # rubocop:enable Metrics/ParameterLists

      def view_template
        Dialog do
          DialogTrigger do
            Button(variant: button_variant, class: button_class) do
              if icon_only
                render Icons::RefreshCw.new(size: 16)
                span(class: 'sr-only') { t('medicines.refill_modal.refill_inventory') }
              elsif button_label
                plain button_label
              else
                t('medicines.refill_modal.refill_inventory')
              end
            end
          end

          DialogContent(size: :md) do
            DialogHeader do
              DialogTitle { t('medicines.refill_modal.title', medicine: medicine.name) }
              DialogDescription { t('medicines.refill_modal.description') }
            end

            DialogMiddle do
              render_form
            end
          end
        end
      end

      private

      def render_form
        form_with(url: refill_medicine_path(medicine), method: :patch, class: 'space-y-4') do
          div(class: 'space-y-2') do
            label(for: 'refill_quantity', class: 'text-sm font-medium') { t('medicines.refill_modal.quantity') }
            input(
              id: 'refill_quantity',
              type: 'number',
              name: 'refill[quantity]',
              required: true,
              value: quantity,
              class: 'w-full rounded-md border border-input bg-background px-3 py-2 text-sm'
            )
          end

          div(class: 'space-y-2') do
            label(for: 'refill_restock_date', class: 'text-sm font-medium') { t('medicines.refill_modal.restock_date') }
            input(
              id: 'refill_restock_date',
              type: 'date',
              name: 'refill[restock_date]',
              required: true,
              value: restock_date_value,
              class: 'w-full rounded-md border border-input bg-background px-3 py-2 text-sm'
            )
          end

          div(class: 'flex justify-end gap-3 pt-2') do
            Button(type: :submit, variant: :primary) { t('medicines.refill_button') }
          end
        end
      end

      def restock_date_value
        restock_date.presence || Date.current.to_s
      end
    end
  end
end
