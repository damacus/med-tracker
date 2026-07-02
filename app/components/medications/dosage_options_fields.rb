# frozen_string_literal: true

module Components
  module Medications
    class DosageOptionsFields < Components::Base
      FREQUENCY_TEMPLATES = [
        {
          label: 'Every morning',
          frequency: 'Every morning',
          max_doses: '1',
          min_hours: '24',
          dose_cycle: 'daily',
          times: '08:00'
        },
        {
          label: 'Every evening',
          frequency: 'Every evening',
          max_doses: '1',
          min_hours: '24',
          dose_cycle: 'daily',
          times: '18:00'
        },
        {
          label: 'Twice daily',
          frequency: 'Twice daily',
          max_doses: '2',
          min_hours: '12',
          dose_cycle: 'daily',
          times: '08:00,20:00'
        },
        {
          label: 'Three times daily',
          frequency: 'Three times daily',
          max_doses: '3',
          min_hours: '8',
          dose_cycle: 'daily',
          times: '08:00,14:00,20:00'
        },
        {
          label: 'Every 4 hours',
          frequency: 'Every 4 hours',
          max_doses: '6',
          min_hours: '4',
          dose_cycle: 'daily',
          times: '08:00,12:00,16:00,20:00'
        },
        {
          label: 'Every 4-6 hours',
          frequency: 'Every 4-6 hours',
          max_doses: '6',
          min_hours: '4',
          dose_cycle: 'daily',
          times: '08:00,14:00,20:00'
        },
        {
          label: 'Once weekly',
          frequency: 'Once weekly',
          max_doses: '1',
          min_hours: '168',
          dose_cycle: 'weekly',
          times: '08:00'
        }
      ].freeze

      attr_reader :medication

      def initialize(medication:)
        @medication = medication
        super()
      end

      def view_template
        div(class: 'space-y-6', data: { controller: 'dosage-options' }) do
          m3_heading(variant: :title_large, level: 3, class: 'font-bold text-foreground') { 'Dose Options' }
          m3_text(variant: :body_medium, class: 'text-on-surface-variant font-medium') do
            'Manage all medication-owned dose options here. Schedules copy these values when they are created.'
          end
          div(class: 'space-y-6') do
            dosage_form_rows.each_with_index do |dosage, index|
              render_dosage_option_fields(dosage, index)
            end
          end
        end
      end

      private

      def dosage_form_rows
        @dosage_form_rows ||= medication.dosage_records.to_a.sort_by do |dosage|
          [dosage.persisted? ? 0 : 1, dosage.amount.to_f, dosage.id || 0]
        end
      end

      def render_dosage_option_fields(dosage, index)
        div(
          class: 'space-y-4 rounded-3xl border border-outline-variant/50 ' \
                 'bg-surface-container-low p-6 shadow-elevation-1',
          data: { 'dosage-options-target': 'option' }
        ) do
          input(type: :hidden, name: dosage_field_name(index, 'id'), value: dosage.id) if dosage.persisted?
          input(
            type: :hidden,
            name: dosage_field_name(index, '_destroy'),
            value: '0',
            data: { 'dosage-options-target': 'destroyField' }
          )

          render_dosage_option_editor(dosage, index)
          render_removed_dosage_option_state
        end
      end

      def render_dosage_option_editor(dosage, index)
        div(
          class: 'space-y-4',
          data: {
            'dosage-options-target': 'editor',
            controller: 'frequency-suggestions'
          }
        ) do
          div(class: 'grid grid-cols-1 sm:grid-cols-2 gap-4') do
            render_dosage_option_amount_field(dosage, index)
            render_dosage_option_unit_field(dosage, index)
          end

          div(class: 'space-y-2') { render_frequency_field(dosage, index) }
          render_description_field(dosage, index)
          render_default_timing_fields(dosage, index)
          render_inventory_fields(dosage, index)
          render_default_flags(dosage, index)
        end
      end

      def render_description_field(dosage, index)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(for: "medication_dosage_records_attributes_#{index}_description") do
            'Description / notes'
          end
          m3_input(
            type: :text,
            name: dosage_field_name(index, 'description'),
            id: "medication_dosage_records_attributes_#{index}_description",
            value: dosage.description,
            placeholder: 'Optional'
          )
        end
      end

      def render_default_timing_fields(dosage, index)
        div(class: 'grid grid-cols-1 sm:grid-cols-3 gap-4') do
          render_dosage_default_number_field(
            dosage: dosage,
            index: index,
            config: {
              field: 'default_max_daily_doses',
              label: 'Max doses / cycle',
              value: dosage.default_max_daily_doses,
              min: '1',
              target: 'maxDoses'
            }
          )
          render_dosage_default_number_field(
            dosage: dosage,
            index: index,
            config: {
              field: 'default_min_hours_between_doses',
              label: 'Min hours apart',
              value: dosage.default_min_hours_between_doses,
              min: '0',
              step: '0.5',
              target: 'minHours'
            }
          )
          render_dosage_default_cycle_field(dosage, index)
        end
      end

      def render_inventory_fields(dosage, index)
        div(class: 'grid grid-cols-1 sm:grid-cols-2 gap-4') do
          render_dosage_inventory_field(
            dosage: dosage,
            index: index,
            field: 'current_supply',
            label: 'Tracked supply'
          )
          render_dosage_inventory_field(
            dosage: dosage,
            index: index,
            field: 'reorder_threshold',
            label: 'Dose stock threshold'
          )
        end
      end

      def render_default_flags(dosage, index)
        div(class: 'flex flex-wrap items-center gap-6') do
          render_dosage_default_checkbox(dosage, index, 'default_for_adults', 'Default for adults')
          render_dosage_default_checkbox(dosage, index, 'default_for_children', 'Default for children / dependents')
          render_remove_dosage_option_button
        end
      end

      def render_frequency_field(dosage, index)
        render RubyUI::FormFieldLabel.new(for: "medication_dosage_records_attributes_#{index}_frequency") do
          'Frequency'
        end
        render_frequency_template_buttons
        m3_input(
          type: :text,
          name: dosage_field_name(index, 'frequency'),
          id: "medication_dosage_records_attributes_#{index}_frequency",
          value: dosage.frequency,
          placeholder: 'Every morning',
          required: dosage_row_requires_input?(dosage),
          data: { 'frequency-suggestions-target': 'input' },
          **field_error_attributes(
            dosage,
            :frequency,
            input_id: "medication_dosage_records_attributes_#{index}_frequency"
          )
        )
        render_field_error(
          dosage,
          :frequency,
          input_id: "medication_dosage_records_attributes_#{index}_frequency"
        )
      end

      def render_frequency_template_buttons
        div(class: 'flex flex-nowrap overflow-x-auto gap-1.5 mt-1 mb-2 pb-0.5 -mx-0.5 px-0.5') do
          FREQUENCY_TEMPLATES.each do |template|
            button(
              type: 'button',
              data: {
                action: 'click->frequency-suggestions#suggest',
                frequency: template.fetch(:label),
                'frequency-suggestions-frequency-value': template.fetch(:frequency),
                'frequency-suggestions-max-doses-value': template.fetch(:max_doses),
                'frequency-suggestions-min-hours-value': template.fetch(:min_hours),
                'frequency-suggestions-dose-cycle-value': template.fetch(:dose_cycle),
                times: template.fetch(:times)
              },
              class: 'inline-flex shrink-0 items-center rounded-full border border-outline-variant/50 ' \
                     'bg-surface-container px-3 py-1 text-xs font-semibold ' \
                     'text-on-surface-variant shadow-elevation-1 ' \
                     'whitespace-nowrap hover:bg-secondary-container hover:text-on-secondary-container ' \
                     'cursor-pointer transition-all'
            ) { template.fetch(:label) }
          end
        end
      end

      def render_remove_dosage_option_button
        button(
          type: 'button',
          class: 'inline-flex items-center gap-2 text-sm font-semibold text-error ' \
                 'hover:text-error/80 transition-colors',
          data: { action: 'click->dosage-options#remove' }
        ) { 'Remove dose option' }
      end

      def render_removed_dosage_option_state
        div(
          class: 'hidden items-center justify-between gap-4 rounded-shape-xl border border-dashed ' \
                 'border-error/30 bg-error-container/10 px-4 py-3',
          data: { 'dosage-options-target': 'removedState' }
        ) do
          m3_text(variant: :body_medium, class: 'font-medium text-on-error-container') { 'Dose option removed' }
          button(
            type: 'button',
            class: 'inline-flex items-center gap-2 text-sm font-semibold text-primary ' \
                   'hover:text-primary/80 transition-colors',
            data: { action: 'click->dosage-options#undo' }
          ) { 'Undo' }
        end
      end

      def render_dosage_option_amount_field(dosage, index)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(for: "medication_dosage_records_attributes_#{index}_amount") do
            'Amount'
          end
          m3_input(
            type: :number,
            name: dosage_field_name(index, 'amount'),
            id: "medication_dosage_records_attributes_#{index}_amount",
            value: dosage.amount&.to_s,
            step: 'any',
            min: '0',
            required: dosage_row_requires_input?(dosage),
            **field_error_attributes(dosage, :amount, input_id: "medication_dosage_records_attributes_#{index}_amount")
          )
          render_field_error(dosage, :amount, input_id: "medication_dosage_records_attributes_#{index}_amount")
        end
      end

      def render_dosage_option_unit_field(dosage, index)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(for: "medication_dosage_records_attributes_#{index}_unit") do
            'Unit'
          end
          m3_select(
            name: dosage_field_name(index, 'unit'),
            id: "medication_dosage_records_attributes_#{index}_unit",
            size: :sm
          ) do
            option(value: '', selected: dosage.unit.blank?) { 'Select unit' }
            Medication::DOSAGE_UNITS.each do |unit|
              option(value: unit, selected: dosage.unit == unit) { unit }
            end
          end
        end
      end

      def render_dosage_inventory_field(dosage:, index:, field:, label:)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(for: "medication_dosage_records_attributes_#{index}_#{field}") do
            label
          end
          m3_input(
            type: :number,
            name: dosage_field_name(index, field),
            id: "medication_dosage_records_attributes_#{index}_#{field}",
            value: inventory_field_value(dosage.public_send(field)),
            min: '0',
            step: '0.01'
          )
        end
      end

      def render_dosage_default_number_field(dosage:, index:, config:)
        field = config.fetch(:field)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(for: "medication_dosage_records_attributes_#{index}_#{field}") do
            config.fetch(:label)
          end
          options = {
            type: :number,
            name: dosage_field_name(index, field),
            id: "medication_dosage_records_attributes_#{index}_#{field}",
            value: config[:value]&.to_s,
            min: config.fetch(:min),
            required: dosage_row_requires_input?(dosage),
            data: { 'frequency-suggestions-target': config.fetch(:target) }
          }
          options[:step] = config[:step] if config[:step]
          options.merge!(
            field_error_attributes(
              dosage,
              field,
              input_id: "medication_dosage_records_attributes_#{index}_#{field}"
            )
          )
          m3_input(**options)
          render_field_error(dosage, field, input_id: "medication_dosage_records_attributes_#{index}_#{field}")
        end
      end

      def render_dosage_default_cycle_field(dosage, index)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(for: "medication_dosage_records_attributes_#{index}_default_dose_cycle") do
            'Dose cycle'
          end
          m3_select(
            name: dosage_field_name(index, 'default_dose_cycle'),
            id: "medication_dosage_records_attributes_#{index}_default_dose_cycle",
            size: :sm,
            required: dosage_row_requires_input?(dosage),
            data: { 'frequency-suggestions-target': 'doseCycle' }
          ) do
            option(value: '', selected: dosage.default_dose_cycle.blank?) { 'Select cycle' }
            MedicationDosage::DOSE_CYCLE_OPTIONS.each do |label, value|
              option(value: value, selected: dosage.default_dose_cycle == value) { label }
            end
          end
          render_field_error(
            dosage,
            :default_dose_cycle,
            input_id: "medication_dosage_records_attributes_#{index}_default_dose_cycle"
          )
        end
      end

      def render_dosage_default_checkbox(dosage, index, field, label)
        input(type: 'hidden', name: dosage_field_name(index, field), value: '0')
        label(class: 'flex items-center gap-2 text-sm cursor-pointer') do
          input(
            type: 'checkbox',
            name: dosage_field_name(index, field),
            value: '1',
            checked: dosage.public_send("#{field}?"),
            class: 'rounded border-outline'
          )
          span { label }
        end
      end

      def dosage_field_name(index, field)
        "medication[dosage_records_attributes][#{index}][#{field}]"
      end

      def dosage_row_requires_input?(dosage)
        return true if dosage.persisted? || dosage.errors.any?

        dosage_row_values(dosage).any?(&:present?) || dosage_row_default_flags(dosage).any?
      end

      def dosage_row_values(dosage)
        [
          dosage.amount,
          dosage.unit,
          dosage.frequency,
          dosage.description,
          dosage.default_max_daily_doses,
          dosage.default_min_hours_between_doses,
          dosage.current_supply,
          dosage.reorder_threshold
        ]
      end

      def dosage_row_default_flags(dosage)
        [
          dosage.default_for_adults?,
          dosage.default_for_children?
        ]
      end

      def inventory_field_value(value)
        return if value.blank?

        MedicationStockQuantityFormatter.format(value)
      end
    end
  end
end
