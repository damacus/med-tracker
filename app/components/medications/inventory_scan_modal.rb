# frozen_string_literal: true

module Components
  module Medications
    class InventoryScanModal < Components::Base
      include Phlex::Rails::Helpers::FormWith

      def view_template
        Dialog do
          DialogTrigger do
            m3_button(
              variant: :outlined,
              size: :lg,
              class: 'rounded-full font-bold text-sm bg-card shadow-sm border-border'
            ) do
              render Icons::Camera.new(size: 20, class: 'mr-2 text-primary')
              span { t('medications.index.scan_stock') }
            end
          end

          DialogContent(size: :md) do
            DialogHeader do
              DialogTitle { t('medications.inventory_scan_modal.title') }
              DialogDescription { t('medications.inventory_scan_modal.description') }
            end

            DialogMiddle do
              div(
                data: {
                  controller: 'inventory-scan',
                  action: 'barcode-scanner:decoded->inventory-scan#barcodeDecoded'
                },
                class: 'space-y-4'
              ) do
                render Components::BarcodeScanner.new
                render_form
              end
            end
          end
        end
      end

      private

      def render_form
        form_with(url: scan_restock_medications_path, method: :post, class: 'space-y-4') do
          render_barcode_field
          render_quantity_field
          div(class: 'flex justify-end gap-3 pt-2') do
            m3_button(type: :submit, variant: :filled) { t('medications.inventory_scan_modal.submit') }
          end
        end
      end

      def render_barcode_field
        div(class: 'space-y-2') do
          label(for: 'inventory_scan_barcode', class: 'text-sm font-medium') do
            t('medications.inventory_scan_modal.barcode')
          end
          input(
            id: 'inventory_scan_barcode',
            type: 'text',
            name: 'inventory_scan[barcode]',
            required: true,
            data: { inventory_scan_target: 'barcode' },
            class: 'w-full rounded-shape-sm border border-outline bg-background px-3 py-2 text-sm ' \
                   'focus:ring-2 focus:ring-primary/20 transition-all'
          )
        end
      end

      def render_quantity_field
        div(class: 'space-y-2') do
          label(for: 'inventory_scan_quantity', class: 'text-sm font-medium') do
            t('medications.inventory_scan_modal.quantity')
          end
          input(
            id: 'inventory_scan_quantity',
            type: 'number',
            name: 'inventory_scan[quantity]',
            required: true,
            min: '0.01',
            step: '0.01',
            class: 'w-full rounded-shape-sm border border-outline bg-background px-3 py-2 text-sm ' \
                   'focus:ring-2 focus:ring-primary/20 transition-all'
          )
        end
      end
    end
  end
end
