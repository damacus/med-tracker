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
          FormFieldLabel(for: 'person_medication_medication_id_trigger') { 'Medication' }
          render RubyUI::Combobox.new(class: 'w-full') do
            render RubyUI::ComboboxTrigger.new(
              placeholder: selected_medication_name || 'Select a medication',
              disabled: editing
            )

            render RubyUI::ComboboxPopover.new do
              render RubyUI::ComboboxSearchInput.new(placeholder: 'Search medications…')

              render RubyUI::ComboboxList.new do
                render(RubyUI::ComboboxEmptyState.new { 'No medications found.' })

                medications.each do |med|
                  render RubyUI::ComboboxItem.new do
                    render RubyUI::ComboboxRadio.new(
                      name: 'person_medication[medication_id]',
                      id: "person_medication_medication_id_#{med.id}",
                      value: med.id,
                      checked: person_medication.medication_id == med.id,
                      required: !editing,
                      data: {
                        text: med.name,
                        person_medication_form_target: 'medicationSelect',
                        action: 'change->person-medication-form#updateDefaults'
                      }
                    )
                    span { med.name }
                  end
                end
              end
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
        div(class: 'grid grid-cols-1 md:grid-cols-3 gap-4') do
          render_max_daily_doses_field
          render_min_hours_field
          render_dose_cycle_field
        end
      end

      def render_max_daily_doses_field
        FormField do
          FormFieldLabel(for: 'person_medication_max_daily_doses') { 'Max doses / cycle' }
          Input(
            type: :number,
            name: 'person_medication[max_daily_doses]',
            id: 'person_medication_max_daily_doses',
            value: person_medication.max_daily_doses,
            min: 1,
            placeholder: 'Optional',
            data: { person_medication_form_target: 'maxDosesInput' }
          )
          FormFieldHint { 'Max doses allowed per cycle' }
        end
      end

      def render_min_hours_field
        FormField do
          FormFieldLabel(for: 'person_medication_min_hours_between_doses') { 'Min hours apart' }
          Input(
            type: :number,
            name: 'person_medication[min_hours_between_doses]',
            id: 'person_medication_min_hours_between_doses',
            value: person_medication.min_hours_between_doses,
            min: 1,
            step: 0.5,
            placeholder: 'Optional',
            data: { person_medication_form_target: 'minHoursInput' }
          )
          FormFieldHint { 'Min time between doses' }
        end
      end

      def selected_medication_name
        return nil if person_medication.medication_id.blank?

        medications.find { |m| m.id == person_medication.medication_id }&.name
      end

      def render_dose_cycle_field
        FormField do
          FormFieldLabel(for: 'person_medication_dose_cycle_trigger') { 'Dose cycle' }
          render RubyUI::Combobox.new(class: 'w-full') do
            render RubyUI::ComboboxTrigger.new(
              placeholder: person_medication.dose_cycle&.titleize || 'Daily'
            )

            render RubyUI::ComboboxPopover.new do
              render RubyUI::ComboboxList.new do
                render(RubyUI::ComboboxEmptyState.new { 'No options.' })

                PersonMedication::DOSE_CYCLE_OPTIONS.each do |label, value|
                  render RubyUI::ComboboxItem.new do
                    render RubyUI::ComboboxRadio.new(
                      name: 'person_medication[dose_cycle]',
                      id: "person_medication_dose_cycle_#{value}",
                      value: value,
                      checked: person_medication.dose_cycle == value,
                      data: { person_medication_form_target: 'doseCycleInput' }
                    )
                    span { label }
                  end
                end
              end
            end
          end
          FormFieldHint { 'Cycle reset period' }
        end
      end
    end
  end
end
