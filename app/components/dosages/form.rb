# frozen_string_literal: true

module Components
  module Dosages
    # Dosage create/edit form
    class Form < Components::Base
      include Phlex::Rails::Helpers::FormWith

      FREQUENCY_SUGGESTIONS = [
        'Once daily',
        'Twice daily',
        'Three times daily',
        'Every 4 hours',
        'Every 4–6 hours',
        'Every 6 hours',
        'Every 8 hours',
        'Every 12 hours',
        'Every morning',
        'Every night',
        'As needed (PRN)'
      ].freeze

      attr_reader :dosage, :medication

      def initialize(dosage:, medication:)
        @dosage = dosage
        @medication = medication
        super()
      end

      def view_template
        form_with(
          model: [medication, dosage],
          class: 'space-y-5'
        ) do |_f|
          render_errors if dosage.errors.any?
          render_basic_fields
          render_divider
          render_scheduling_defaults
          render_default_flags
          render_actions
        end
      end

      private

      def render_errors
        Alert(variant: :destructive, class: 'mb-2') do
          AlertTitle { "#{dosage.errors.count} error(s):" }
          AlertDescription do
            ul(class: 'list-disc ml-4') do
              dosage.errors.full_messages.each { |m| li { m } }
            end
          end
        end
      end

      def render_basic_fields
        div(class: 'grid grid-cols-2 gap-4') do
          FormField do
            FormFieldLabel(for: 'dosage_amount') do
              plain 'Amount'
              span(class: 'text-destructive ml-0.5') { ' *' }
            end
            Input(type: :number, name: 'dosage[amount]', id: 'dosage_amount',
                  value: dosage.amount, step: 'any', min: '0', required: true,
                  **field_error_attributes(dosage, :amount, input_id: 'dosage_amount'))
            render_field_error(dosage, :amount, input_id: 'dosage_amount')
          end

          FormField do
            FormFieldLabel(for: 'dosage_unit') do
              plain 'Unit'
              span(class: 'text-destructive ml-0.5') { ' *' }
            end
            Input(type: :text, name: 'dosage[unit]', id: 'dosage_unit',
                  value: dosage.unit, required: true,
                  placeholder: 'mg, tablet, ml…',
                  list: 'dosage_unit_list',
                  **field_error_attributes(dosage, :unit, input_id: 'dosage_unit'))
            render_field_error(dosage, :unit, input_id: 'dosage_unit')
            datalist(id: 'dosage_unit_list') do
              Medication::DOSAGE_UNITS.each { |u| option(value: u) }
            end
          end
        end

        FormField(class: 'mt-4', data: { controller: 'frequency-suggestions' }) do
          FormFieldLabel(for: 'dosage_frequency') do
            plain 'Frequency label'
            span(class: 'text-destructive ml-0.5') { ' *' }
          end
          FormFieldHint { 'Short description, e.g. "Once daily", "Every 4–6 hours"' }
          render_frequency_suggestions
          Input(type: :text, name: 'dosage[frequency]', id: 'dosage_frequency',
                value: dosage.frequency, required: true,
                placeholder: 'Once daily',
                data: { 'frequency-suggestions-target': 'input' },
                **field_error_attributes(dosage, :frequency, input_id: 'dosage_frequency'))
          render_field_error(dosage, :frequency, input_id: 'dosage_frequency')
        end

        FormField(class: 'mt-4') do
          FormFieldLabel(for: 'dosage_description') { 'Description / notes' }
          Input(type: :text, name: 'dosage[description]', id: 'dosage_description',
                value: dosage.description, placeholder: 'Optional')
        end
      end

      def render_frequency_suggestions
        div(class: 'flex flex-nowrap overflow-x-auto gap-1.5 mt-1 mb-2 pb-0.5 -mx-0.5 px-0.5') do
          FREQUENCY_SUGGESTIONS.each do |suggestion|
            button(
              type: 'button',
              data: {
                action: 'click->frequency-suggestions#suggest',
                suggestion: suggestion
              },
              class: 'inline-flex shrink-0 items-center rounded-full border ' \
                     'border-border bg-surface-container-lowest px-2.5 py-0.5 ' \
                     'text-xs font-medium text-muted-foreground shadow-sm ' \
                     'whitespace-nowrap hover:bg-accent hover:border-border ' \
                     'cursor-pointer transition-colors'
            ) { suggestion }
          end
        end
      end

      def render_divider
        div(class: 'border-t border-border my-2')
        p(class: 'text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-1') do
          'Scheduling defaults (auto-filled when creating a schedule)'
        end
      end

      def render_scheduling_defaults
        div(class: 'grid grid-cols-3 gap-4') do
          FormField do
            FormFieldLabel(for: 'dosage_default_max_daily_doses') { 'Max doses / cycle' }
            Input(type: :number, name: 'dosage[default_max_daily_doses]',
                  id: 'dosage_default_max_daily_doses',
                  placeholder: 'e.g., 4',
                  value: dosage.default_max_daily_doses, min: 1)
          end

          FormField do
            FormFieldLabel(for: 'dosage_default_min_hours_between_doses') { 'Min hours apart' }
            Input(type: :number, name: 'dosage[default_min_hours_between_doses]',
                  id: 'dosage_default_min_hours_between_doses',
                  placeholder: 'e.g., 6',
                  value: dosage.default_min_hours_between_doses, min: 0, step: '0.5')
          end

          FormField do
            FormFieldLabel(for: 'dosage_default_dose_cycle') { 'Dose cycle' }
            select(
              name: 'dosage[default_dose_cycle]',
              id: 'dosage_default_dose_cycle',
              class: 'flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm'
            ) do
              option(value: '') { '— none —' }
              Dosage::DOSE_CYCLE_OPTIONS.each do |label, value|
                option(value: value, selected: dosage.default_dose_cycle == value) { label }
              end
            end
          end
        end
      end

      def render_default_flags
        div(class: 'flex gap-6 mt-2') do
          label(class: 'flex items-center gap-2 text-sm cursor-pointer') do
            input(type: 'checkbox', name: 'dosage[default_for_adults]', value: '1',
                  checked: dosage.default_for_adults?,
                  class: 'rounded border-input')
            span { 'Default for adults' }
          end

          label(class: 'flex items-center gap-2 text-sm cursor-pointer') do
            input(type: 'checkbox', name: 'dosage[default_for_children]', value: '1',
                  checked: dosage.default_for_children?,
                  class: 'rounded border-input')
            span { 'Default for children / dependents' }
          end
        end
      end

      def render_actions
        div(class: 'flex gap-3 justify-end pt-2') do
          Button(variant: :ghost, data: { action: 'click->ruby-ui--dialog#dismiss' }) { 'Cancel' }
          Button(type: :submit, variant: :primary) do
            dosage.new_record? ? 'Add Dosage' : 'Update Dosage'
          end
        end
      end
    end
  end
end
