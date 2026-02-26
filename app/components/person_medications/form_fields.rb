# frozen_string_literal: true

module Components
  module PersonMedications
    # Shared form fields component for person medication forms
    # Used by both FormView and Modal components
    class FormFields < Components::Base
      attr_reader :person_medication, :medications, :editing

      def initialize(person_medication:, medications:, editing: false)
        @person_medication = person_medication
        @medications = medications
        @editing = editing
        super()
      end

      def view_template
        div(class: 'space-y-4') do
          render_medication_field
          render_notes_field
          render_timing_fields
        end
      end

      private

      def render_medication_field
        FormField do
          FormFieldLabel(for: 'person_medication_medication_id') { 'Medication' }
          select(
            name: 'person_medication[medication_id]',
            id: 'person_medication_medication_id',
            required: !editing,
            disabled: editing,
            class: select_classes
          ) do
            option(value: '', disabled: true, selected: person_medication.medication_id.blank?) { 'Select a medication' }
            medications.each do |medication|
              option(value: medication.id, selected: person_medication.medication_id == medication.id) { medication.name }
            end
          end
          FormFieldHint { 'Select a medication from the list' }
        end
      end

      def render_notes_field
        FormField do
          FormFieldLabel(for: 'person_medication_notes') { 'Notes' }
          Textarea(
            name: 'person_medication[notes]',
            id: 'person_medication_notes',
            placeholder: 'Add any special instructions or notes',
            rows: 3
          ) { person_medication.notes }
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
          FormFieldLabel(for: 'person_medication_max_daily_doses') { 'Max daily doses' }
          Input(
            type: :number,
            name: 'person_medication[max_daily_doses]',
            id: 'person_medication_max_daily_doses',
            value: person_medication.max_daily_doses,
            min: 1,
            placeholder: 'Optional'
          )
          FormFieldHint { 'Maximum doses per day' }
        end
      end

      def render_min_hours_field
        FormField do
          FormFieldLabel(for: 'person_medication_min_hours_between_doses') { 'Min hours between doses' }
          Input(
            type: :number,
            name: 'person_medication[min_hours_between_doses]',
            id: 'person_medication_min_hours_between_doses',
            value: person_medication.min_hours_between_doses,
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
