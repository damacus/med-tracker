# frozen_string_literal: true

module Components
  module Medications
    class SupplyFields < Components::Base
      attr_reader :medication

      def initialize(medication:)
        @medication = medication
        super()
      end

      def view_template
        div(class: 'grid grid-cols-1 sm:grid-cols-2 gap-6') do
          render_current_supply_field
          render_reorder_threshold_field
        end
      end

      private

      def render_current_supply_field
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'medication_current_supply',
            class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
          ) { t('forms.medications.current_supply') }
          m3_input(
            type: :number,
            name: 'medication[current_supply]',
            id: 'medication_current_supply',
            value: inventory_field_value(medication.current_supply),
            min: '0',
            step: '0.01',
            placeholder: t('forms.medications.current_supply_placeholder', default: 'e.g., 30'),
            class: 'rounded-shape-sm border-outline-variant bg-surface-container-lowest py-4 px-4 ' \
                   'focus:ring-2 focus:ring-primary/10 ' \
                   'focus:border-primary transition-all'
          )
        end
      end

      def render_reorder_threshold_field
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'medication_reorder_threshold',
            class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
          ) { t('forms.medications.reorder_threshold') }
          m3_input(
            type: :number,
            name: 'medication[reorder_threshold]',
            id: 'medication_reorder_threshold',
            value: inventory_field_value(medication.reorder_threshold),
            min: '0',
            step: '0.01',
            placeholder: t('forms.medications.reorder_threshold_placeholder', default: 'e.g., 5'),
            class: 'rounded-shape-sm border-outline-variant bg-surface-container-lowest py-4 px-4 ' \
                   'focus:ring-2 focus:ring-primary/10 ' \
                   'focus:border-primary transition-all'
          )
        end
      end

      def inventory_field_value(value)
        return if value.blank?

        MedicationStockQuantityFormatter.format(value)
      end
    end
  end
end
