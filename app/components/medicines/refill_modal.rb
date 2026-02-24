# frozen_string_literal: true

module Components
  module Medicines
    class RefillModal < Components::Base
      include Phlex::Rails::Helpers::FormWith

      attr_reader :medicine, :button_variant, :button_class, :quantity, :restock_date, :icon_only

      def initialize(medicine:, button_variant: :outline, button_class: '', quantity: nil, restock_date: nil,
                     icon_only: false)
        @medicine = medicine
        @button_variant = button_variant
        @button_class = button_class
        @quantity = quantity
        @restock_date = restock_date
        @icon_only = icon_only
        super()
      end

      def view_template
        Dialog do
          DialogTrigger do
            Button(variant: button_variant, class: button_class) do
              if icon_only
                svg(
                  xmlns: 'http://www.w3.org/2000/svg',
                  class: 'w-4 h-4',
                  fill: 'none',
                  viewBox: '0 0 24 24',
                  stroke: 'currentColor',
                  aria: { hidden: 'true' }
                ) do |s|
                  s.path(
                    stroke_linecap: 'round',
                    stroke_linejoin: 'round',
                    stroke_width: '2',
                    d: 'M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 ' \
                       '8.003 0 01-15.357-2m15.357 2H15'
                  )
                end
                span(class: 'sr-only') { 'Refill Inventory' }
              else
                'Refill Inventory'
              end
            end
          end

          DialogContent(size: :md) do
            DialogHeader do
              DialogTitle { "Refill #{medicine.name}" }
              DialogDescription { 'Add supply quantity and record the restock date.' }
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
            label(for: 'refill_quantity', class: 'text-sm font-medium') { 'Quantity' }
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
            label(for: 'refill_restock_date', class: 'text-sm font-medium') { 'Restock date' }
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
            Button(type: :submit, variant: :primary) { 'Save Refill' }
          end
        end
      end

      def restock_date_value
        restock_date.presence || Date.current.to_s
      end
    end
  end
end
