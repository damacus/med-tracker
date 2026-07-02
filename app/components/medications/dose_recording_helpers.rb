# frozen_string_literal: true

module Components
  module Medications
    module DoseRecordingHelpers # rubocop:disable Metrics/ModuleLength
      extend ActiveSupport::Concern

      private

      def render_context
        div(class: 'rounded-shape-xl border border-border/70 bg-surface-container-low p-4') do
          dl(class: 'grid gap-3 sm:grid-cols-2') do
            div(class: 'space-y-1') do
              dt(class: 'text-xs font-black uppercase text-on-surface-variant') do
                t('medications.take_action.person', default: 'Person')
              end
              dd(class: 'text-sm font-semibold text-foreground') { person.name }
            end
            div(class: 'space-y-1') do
              dt(class: 'text-xs font-black uppercase text-on-surface-variant') do
                t('medications.take_action.medication', default: 'Medication')
              end
              dd(class: 'text-sm font-semibold text-foreground') { source.medication.display_name }
            end
            div(class: 'space-y-1 sm:col-span-2') do
              dt(class: 'text-xs font-black uppercase text-on-surface-variant') do
                t('medications.take_action.dose', default: 'Dose')
              end
              dd(class: 'text-sm font-semibold text-foreground') { dose_label }
            end
          end
        end
      end

      def render_stock_source_selection
        if available_medications.many?
          render_stock_source_options
        elsif available_medications.one?
          input(type: :hidden, name: 'medication_take[taken_from_medication_id]', value: available_medications.first.id)
          render_selected_stock_source(available_medications.first)
        end
      end

      def render_stock_source_options
        fieldset(class: 'space-y-2') do
          legend(class: 'text-sm font-semibold text-foreground') do
            t('medications.take_action.stock_source', default: 'Stock source')
          end
          available_medications.each_with_index do |medication, index|
            render_stock_source_option(medication, index)
          end
        end
      end

      def render_stock_source_option(medication, index)
        label(
          for: stock_source_input_id(medication),
          class: 'flex flex-col gap-3 rounded-shape-xl border border-border p-4 sm:flex-row ' \
                 'sm:items-center sm:justify-between'
        ) do
          render_stock_source_details(medication)
          div(class: 'flex items-center gap-3 shrink-0') do
            Badge(variant: :outlined, class: 'rounded-full text-[10px] whitespace-nowrap justify-center') do
              inventory_label(medication)
            end
            input(
              type: :radio,
              id: stock_source_input_id(medication),
              name: 'medication_take[taken_from_medication_id]',
              value: medication.id,
              checked: index.zero?
            )
          end
        end
      end

      def render_selected_stock_source(medication)
        div(class: 'space-y-2') do
          m3_text(variant: :label_medium, class: 'font-semibold text-foreground') do
            t('medications.take_action.stock_source', default: 'Stock source')
          end
          div(class: 'rounded-shape-xl border border-border p-4') do
            render_stock_source_details(medication)
          end
        end
      end

      def render_stock_source_details(medication)
        div(class: 'space-y-1 min-w-0') do
          m3_text(size: '2', weight: 'bold', class: 'text-foreground') { medication.location.name }
          m3_text(size: '1', class: 'text-on-surface-variant break-words') do
            medication_description(medication)
          end
        end
      end

      def take_path
        if source.is_a?(::Schedule)
          take_medication_person_schedule_path(person, source)
        else
          take_medication_person_person_medication_path(person, source)
        end
      end

      def offline_source_type
        source.is_a?(::Schedule) ? 'schedule' : 'person_medication'
      end

      def available_medications
        @available_medications ||= stock_source_resolver.available_medications
      end

      def medication_description(medication)
        parts = [medication.display_name]
        dose = DoseAmount.new(medication.dose_amount, medication.dose_unit).to_s
        parts << dose if dose.present?
        parts.join(' • ')
      end

      def inventory_label(medication)
        if medication.current_supply.blank?
          return t('medications.take_action.untracked_inventory', default: 'Untracked')
        end

        stock_label_for(medication)
      end

      def stock_label_for(medication)
        if medication.dose_unit == 'ml'
          return "#{MedicationStockQuantityFormatter.format(medication.current_supply)} ml"
        end

        supply = MedicationStockQuantityFormatter.format(medication.current_supply)
        supply == '1' ? '1 unit' : "#{supply} units"
      end

      def formatted_amount
        amount.to_s
      end

      def dose_label
        dose = DoseAmount.new(amount, source_dose_unit).to_s
        dose.presence || formatted_amount
      end

      def source_dose_unit
        return source.dose_unit if source.respond_to?(:dose_unit)

        source.medication.dose_unit
      end

      def stock_source_input_id(medication)
        "taken_from_medication_#{source.class.name.underscore}_#{source.id}_#{medication.id}"
      end

      def stock_source_resolver
        @stock_source_resolver ||= MedicationStockSourceResolver.new(user: current_user, source: source)
      end
    end
  end
end
