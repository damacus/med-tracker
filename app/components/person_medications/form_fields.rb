# frozen_string_literal: true

module Components
  module PersonMedications
    # Shared form fields component for person medication forms
    # Used by both FormView and Modal components
    class FormFields < Components::Base
      attr_reader :person_medication, :medications, :editing, :workflow

      def initialize(person_medication:, medications:, editing: false, workflow: false)
        @person_medication = person_medication
        @medications = medications
        @editing = editing
        @workflow = workflow
        super()
      end

      def view_template
        if workflow
          render_workflow
        else
          div(class: 'space-y-4') do
            render_medication_field
            render_dose_field
            render_notes_field
            render_timing_fields
          end
        end
      end

      private

      def render_workflow
        div(class: 'space-y-6') do
          div(class: 'flex items-center justify-center gap-2') do
            [1, 2, 3].each do |step_number|
              div(class: workflow_progress_classes(step_number),
                  data: { person_medication_form_target: 'stepIndicator' })
            end
          end

          render_workflow_step(
            1,
            title: 'Choose a medication',
            description: 'Start by choosing the medication you want to add.'
          ) do
            render_medication_field
          end

          render_workflow_step(
            2,
            title: 'Choose the dose',
            description: 'Every as-needed medication needs a dose before you continue.'
          ) do
            render_selection_summary
            render_dose_field
          end

          render_workflow_step(
            3,
            title: 'Add optional guidance',
            description: 'Add any notes or dose limits you want to track with this medication.'
          ) do
            render_selection_summary(show_dose: true)
            render_notes_field
            render_timing_fields
          end
        end
      end

      def render_workflow_step(step_number, title:, description:, &)
        div(class: workflow_step_classes(step_number),
            data: { person_medication_form_target: 'stepPanel', step: step_number }) do
          div(class: 'space-y-1') do
            Heading(level: 2, size: '4', class: 'font-semibold') { title }
            Text(size: '2', class: 'text-slate-500') { description }
          end
          div(class: 'space-y-4', &)
        end
      end

      def render_selection_summary(show_dose: false)
        div(class: selection_summary_layout_classes(show_dose)) do
          div(class: 'rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3') do
            Text(size: '1', weight: 'medium', class: 'uppercase tracking-[0.2em] text-slate-400') { 'Medication' }
            Text(size: '3', weight: 'semibold',
                 data: { person_medication_form_target: 'selectedMedicationName' }) do
              selected_medication_name || 'Choose a medication'
            end
          end

          if show_dose
            div(class: 'rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3') do
              Text(size: '1', weight: 'medium', class: 'uppercase tracking-[0.2em] text-slate-400') { 'Dose' }
              Text(size: '3', weight: 'semibold', data: { person_medication_form_target: 'selectedDoseName' }) do
                selected_dose_label || 'Choose a dose'
              end
            end
          end
        end
      end

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
                        dose_amount: decimal_string(med.dosage_amount),
                        dose_unit: med.dosage_unit,
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

      def render_dose_field
        FormField do
          FormFieldLabel(for: 'person_medication_dose_option') do
            plain 'Dose'
            span(class: 'text-destructive ml-0.5') { ' *' }
          end
          input(
            type: :hidden,
            name: 'person_medication[dose_amount]',
            value: decimal_string(person_medication.dose_amount),
            data: { person_medication_form_target: 'doseAmountInput' }
          )
          input(
            type: :hidden,
            name: 'person_medication[dose_unit]',
            value: person_medication.dose_unit,
            data: { person_medication_form_target: 'doseUnitInput' }
          )
          select(
            id: 'person_medication_dose_option',
            name: 'dose_option',
            required: true,
            disabled: person_medication.medication_id.blank?,
            class: 'w-full rounded-md border border-input bg-background px-3 py-2 text-sm',
            data: { person_medication_form_target: 'doseOptionInput',
                    action: 'change->person-medication-form#selectDose' }
          ) do
            option(value: '') do
              person_medication.medication_id.present? ? 'Select a dose' : 'Select a medication first'
            end
            if person_medication.dose_amount.present? && person_medication.dose_unit.present?
              option(value: selected_dose_option_value, selected: true) { selected_dose_label }
            end
          end
          FormFieldHint { 'All as-needed medications require a dose' }
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

      def selected_dose_option_value
        "#{decimal_string(person_medication.dose_amount)}|#{person_medication.dose_unit}"
      end

      def selected_dose_label
        return if person_medication.dose_amount.blank? || person_medication.dose_unit.blank?

        "#{person_medication.dose_amount.to_f.to_s.sub(/\.0$/, '')} #{person_medication.dose_unit}"
      end

      def selection_summary_layout_classes(show_dose)
        if show_dose
          'grid grid-cols-1 md:grid-cols-2 gap-3'
        else
          'max-w-md'
        end
      end

      def initial_step
        if person_medication.errors.any? && workflow_error_attributes.none? do |key|
             %i[medication dose_amount dose_unit medication_id].include?(key)
           end
          return 3
        end
        return 2 if person_medication.errors[:dose_amount].any? || person_medication.errors[:dose_unit].any?
        return 2 if person_medication.medication_id.present?

        1
      end

      def workflow_step_classes(step_number)
        classes = ['space-y-4']
        classes << 'hidden' if initial_step != step_number
        classes.join(' ')
      end

      def workflow_progress_classes(step_number)
        classes = %w[h-2 w-10 rounded-full transition-colors]
        classes << (step_number <= initial_step ? 'bg-slate-900' : 'bg-slate-200')
        classes.join(' ')
      end

      def decimal_string(value)
        return if value.blank?

        value.is_a?(BigDecimal) ? value.to_s('F') : value.to_s
      end

      def workflow_error_attributes
        person_medication.errors.attribute_names.map(&:to_sym).uniq
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
