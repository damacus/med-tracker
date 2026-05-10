# frozen_string_literal: true

module Components
  module Medications
    class AdjustInventoryModal < Components::Base
      include Phlex::Rails::Helpers::FormWith

      attr_reader :medication, :button_variant, :button_class, :button_size, :button_label

      def initialize(medication:, button_variant: :outlined, button_class: "", button_size: :md, button_label: nil)
        @medication = medication
        @button_variant = button_variant
        @button_class = button_class
        @button_size = button_size
        @button_label = button_label
        super()
      end

      def view_template
        Dialog do
          DialogTrigger do
            m3_button(variant: button_variant, size: button_size, class: button_class) do
              render(Icons::Pencil.new(size: 18, class: "mr-2 text-primary"))
              span { button_label || t("medications.adjust_inventory_modal.adjust_inventory") }
            end
          end

          DialogContent(size: :md) do
            DialogHeader do
              DialogTitle { t("medications.adjust_inventory_modal.title", medication: medication.name) }
              DialogDescription { t("medications.adjust_inventory_modal.description") }
            end

            DialogMiddle do
              render_form
            end
          end
        end
      end

      private

      def render_form
        form_with(url: adjust_inventory_medication_path(medication), method: :patch, class: "space-y-4") do
          render_current_supply_display
          render_new_quantity_field
          render_reason_field
          render_submit_button
        end
      end

      def render_current_supply_display
        div(class: "rounded-shape-sm border border-outline/40 bg-secondary-container/30 px-3 py-2") do
          div(class: "text-xs font-medium text-on-surface-variant mb-0.5") do
            t("medications.adjust_inventory_modal.current_quantity")
          end

          div(class: "text-sm font-bold") do
            plain(MedicationStockQuantityFormatter.format(medication.current_supply || 0))
            plain(" #{t("medications.adjust_inventory_modal.units")}")
          end
        end
      end

      def render_new_quantity_field
        div(class: "space-y-2") do
          label(for: "adjustment_new_quantity", class: "text-sm font-medium") do
            t("medications.adjust_inventory_modal.new_quantity")
          end

          input(
            id: "adjustment_new_quantity",
            type: "number",
            name: "adjustment[new_quantity]",
            required: true,
            min: "0",
            step: "0.01",
            class: "w-full rounded-shape-sm border border-outline bg-background px-3 py-2 text-sm " \
              "focus:ring-2 focus:ring-primary/20 transition-all"
          )
        end
      end

      def render_reason_field
        div(class: "space-y-2") do
          label(for: "adjustment_reason", class: "text-sm font-medium") do
            t("medications.adjust_inventory_modal.reason")
          end

          input(
            id: "adjustment_reason",
            type: "text",
            name: "adjustment[reason]",
            placeholder: t("medications.adjust_inventory_modal.reason_placeholder"),
            class: "w-full rounded-shape-sm border border-outline bg-background px-3 py-2 text-sm " \
              "focus:ring-2 focus:ring-primary/20 transition-all"
          )
        end
      end

      def render_submit_button
        div(class: "flex justify-end gap-3 pt-2") do
          m3_button(type: :submit, variant: :filled) { t("medications.adjust_inventory_modal.submit") }
        end
      end
    end
  end
end
