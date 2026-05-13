# frozen_string_literal: true

module Components
  module Medications
    class InventoryScanModal < Components::Base
      include Phlex::Rails::Helpers::FormWith

      def view_template
        Dialog do
          DialogTrigger do
            m3_button(
              variant: :elevated,
              size: :lg,
              class: 'max-w-full justify-center font-bold text-sm'
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
                  inventory_scan_match_url_value: '/medications/scan_restock_match.json',
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
          render_match_feedback
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
            data: {
              inventory_scan_target: 'barcode',
              action: 'input->inventory-scan#barcodeChanged change->inventory-scan#barcodeChanged'
            },
            class: 'w-full rounded-shape-sm border border-outline bg-background px-3 py-2 text-sm ' \
                   'focus:ring-2 focus:ring-primary/20 transition-all'
          )
        end
      end

      def render_match_feedback
        div(
          hidden: true,
          data: {
            testid: 'inventory-scan-match',
            inventory_scan_target: 'matchPanel'
          },
          class: 'rounded-shape-sm border border-primary/30 bg-primary-container/40 p-3 text-sm ' \
                 'text-on-primary-container'
        ) do
          div(
            data: { inventory_scan_target: 'matchName' },
            class: 'font-semibold'
          )
          dl(class: 'mt-2 grid grid-cols-2 gap-x-3 gap-y-1') do
            dt(class: 'text-on-surface-variant') { t('medications.show.location') }
            dd(data: { inventory_scan_target: 'matchLocation' }, class: 'font-medium text-right')
            dt(class: 'text-on-surface-variant') { t('forms.medications.current_supply', default: 'Current supply') }
            dd(data: { inventory_scan_target: 'matchSupply' }, class: 'font-medium text-right')
          end
        end

        div(
          hidden: true,
          role: 'status',
          data: {
            testid: 'inventory-scan-no-match',
            inventory_scan_target: 'noMatchPanel'
          },
          class: 'rounded-shape-sm border border-outline bg-surface-container-low p-3 text-sm text-on-surface-variant'
        ) do
          t('medications.scan_restock_no_match')
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
