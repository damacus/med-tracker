# frozen_string_literal: true

module Components
  module PersonMedicines
    # Shared form fields component for person medicine forms
    # Used by both FormView and Modal components
    class FormFields < Components::Base
      attr_reader :person_medicine, :medicines

      def initialize(person_medicine:, medicines:)
        @person_medicine = person_medicine
        @medicines = medicines
        super()
      end

      def view_template
        div(class: 'space-y-4') do
          render_medicine_field
          render_notes_field
          render_timing_fields
        end
      end

      private

      def render_medicine_field
        FormField do
          FormFieldLabel(for: 'person_medicine_medicine_id') { 'Medicine' }
          select(
            name: 'person_medicine[medicine_id]',
            id: 'person_medicine_medicine_id',
            required: true,
            class: select_classes
          ) do
            option(value: '', disabled: true, selected: person_medicine.medicine_id.blank?) { 'Select a medicine' }
            medicines.each do |medicine|
              option(value: medicine.id, selected: person_medicine.medicine_id == medicine.id) { medicine.name }
            end
          end
          FormFieldHint { 'Select a medicine from the list' }
        end
      end

      def select_classes
        'flex h-9 w-full items-center justify-between rounded-md border border-input ' \
          'bg-transparent px-3 py-2 text-sm shadow-sm ring-offset-background ' \
          'focus:outline-none focus:ring-1 focus:ring-ring disabled:cursor-not-allowed disabled:opacity-50'
      end

      def render_notes_field
        FormField do
          FormFieldLabel(for: 'person_medicine_notes') { 'Notes' }
          Textarea(
            name: 'person_medicine[notes]',
            id: 'person_medicine_notes',
            placeholder: 'Add any special instructions or notes',
            rows: 3
          ) { person_medicine.notes }
          FormFieldHint { 'Add any special instructions or notes' }
        end
      end

      def render_timing_fields
        div(class: 'grid grid-cols-2 gap-4') do
          render_max_daily_doses_field
          render_min_hours_field
        end
      end

      def render_max_daily_doses_field
        FormField do
          FormFieldLabel(for: 'person_medicine_max_daily_doses') { 'Max daily doses' }
          Input(
            type: :number,
            name: 'person_medicine[max_daily_doses]',
            id: 'person_medicine_max_daily_doses',
            value: person_medicine.max_daily_doses,
            min: 1,
            placeholder: 'Optional'
          )
          FormFieldHint { 'Maximum doses per day' }
        end
      end

      def render_min_hours_field
        FormField do
          FormFieldLabel(for: 'person_medicine_min_hours_between_doses') { 'Min hours between doses' }
          Input(
            type: :number,
            name: 'person_medicine[min_hours_between_doses]',
            id: 'person_medicine_min_hours_between_doses',
            value: person_medicine.min_hours_between_doses,
            min: 1,
            step: 0.5,
            placeholder: 'Optional'
          )
          FormFieldHint { 'Minimum time between doses' }
        end
      end

    end
  end
end
