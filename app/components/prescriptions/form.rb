# frozen_string_literal: true

module Components
  module Prescriptions
    # Renders a prescription form using RubyUI components
    class Form < Components::Base
      include Phlex::Rails::Helpers::FormWith

      attr_reader :prescription, :person, :medicines

      def initialize(prescription:, person:, medicines:)
        @prescription = prescription
        @person = person
        @medicines = medicines
        super()
      end

      def view_template
        form_with(
          model: [person, prescription],
          class: 'space-y-6',
          data: { controller: 'prescription-form' }
        ) do |f|
          render_errors if prescription.errors.any?
          render_form_fields(f)
          render_actions(f)
        end
      end

      private

      def render_errors
        render Components::Shared::ErrorSummary.new(model: prescription, resource_name: 'prescription')
      end

      def render_form_fields(f)
        div(class: 'grid grid-cols-1 md:grid-cols-2 gap-6') do
          render_medicine_field(f)
          render_dosage_field(f)
          render_frequency_field(f)
          render_start_date_field(f)
          render_end_date_field(f)
          render_max_doses_field(f)
          render_min_hours_field(f)
          render_dose_cycle_field(f)
          render_notes_field(f)
        end
      end

      def render_medicine_field(_f)
        FormField do
          FormFieldLabel(for: 'prescription_medicine_id') { 'Medicine' }
          Select(data: { prescription_form_target: 'medicineSelect', testid: 'medicine-select' }) do
            SelectInput(
              name: 'prescription[medicine_id]',
              id: 'prescription_medicine_id',
              value: prescription.medicine_id,
              required: true,
              data: { action: 'change->prescription-form#updateDosages' }
            )
            SelectTrigger(data: { testid: 'medicine-trigger' }) do
              SelectValue(placeholder: 'Select a medicine') do
                prescription.medicine&.name || 'Select a medicine'
              end
            end
            SelectContent do
              medicines.each do |medicine|
                SelectItem(value: medicine.id.to_s) { medicine.name }
              end
            end
          end
        end
      end

      def render_dosage_field(_f)
        FormField do
          FormFieldLabel(for: 'prescription_dosage_id') { 'Dosage' }
          Select(data: { prescription_form_target: 'dosageSelect', testid: 'dosage-select' }) do
            SelectInput(
              name: 'prescription[dosage_id]',
              id: 'prescription_dosage_id',
              value: prescription.dosage_id,
              required: true,
              data: { action: 'change->prescription-form#validate' }
            )
            SelectTrigger(
              disabled: prescription.medicine.nil?,
              aria_disabled: prescription.medicine.nil?,
              data: { testid: 'dosage-trigger', prescription_form_target: 'dosageTrigger' }
            ) do
              SelectValue(
                placeholder: 'Select a medicine first',
                data: { prescription_form_target: 'dosageValue' }
              ) do
                if prescription.dosage
                  format_dosage_option(prescription.dosage)
                else
                  'Select a medicine first'
                end
              end
            end
            SelectContent(data: { prescription_form_target: 'dosageContent' }) do
              (prescription.medicine&.dosages || []).each do |dosage|
                SelectItem(value: dosage.id.to_s) do
                  format_dosage_option(dosage)
                end
              end
            end
          end
        end
      end

      def render_frequency_field(_f)
        FormField do
          FormFieldLabel(for: 'prescription_frequency') { 'Frequency' }
          Input(
            type: :text,
            name: 'prescription[frequency]',
            id: 'prescription_frequency',
            value: prescription.frequency,
            placeholder: 'e.g., Once daily, Every 4-6 hours',
            data: { action: 'input->prescription-form#validate' }
          )
        end
      end

      def render_start_date_field(_f)
        FormField do
          FormFieldLabel(for: 'prescription_start_date') { 'Start date' }
          Input(
            type: :string,
            name: 'prescription[start_date]',
            id: 'prescription_start_date',
            value: prescription.start_date&.to_fs(:db),
            required: true,
            placeholder: 'Select a date',
            data: {
              controller: 'ruby-ui--calendar-input',
              action: 'input->prescription-form#validate'
            }
          )
          Calendar(
            input_id: '#prescription_start_date',
            date_format: 'yyyy-MM-dd',
            class: 'rounded-md border shadow'
          )
        end
      end

      def render_end_date_field(_f)
        FormField do
          FormFieldLabel(for: 'prescription_end_date') { 'End date' }
          Input(
            type: :string,
            name: 'prescription[end_date]',
            id: 'prescription_end_date',
            value: prescription.end_date&.to_fs(:db),
            placeholder: 'Select a date',
            data: { controller: 'ruby-ui--calendar-input' }
          )
          Calendar(
            input_id: '#prescription_end_date',
            date_format: 'yyyy-MM-dd',
            class: 'rounded-md border shadow'
          )
        end
      end

      def render_max_doses_field(_f)
        FormField do
          FormFieldLabel(for: 'prescription_max_daily_doses') { 'Max doses per cycle' }
          FormFieldHint { 'Maximum number of doses allowed per cycle' }
          Input(
            type: :number,
            name: 'prescription[max_daily_doses]',
            id: 'prescription_max_daily_doses',
            value: prescription.max_daily_doses,
            min: 1
          )
        end
      end

      def render_min_hours_field(_f)
        FormField do
          FormFieldLabel(for: 'prescription_min_hours_between_doses') { 'Minimum hours between doses' }
          FormFieldHint { 'Minimum time required between doses' }
          Input(
            type: :number,
            name: 'prescription[min_hours_between_doses]',
            id: 'prescription_min_hours_between_doses',
            value: prescription.min_hours_between_doses,
            min: 1
          )
        end
      end

      def render_dose_cycle_field(_f)
        FormField do
          FormFieldLabel(for: 'prescription_dose_cycle') { 'Dose cycle' }
          FormFieldHint { 'How often the dose cycle resets (default: daily)' }
          Select do
            SelectInput(
              name: 'prescription[dose_cycle]',
              id: 'prescription_dose_cycle',
              value: prescription.dose_cycle
            )
            SelectTrigger do
              SelectValue(placeholder: 'Select a cycle (default: daily)') do
                prescription.dose_cycle&.titleize || 'Select a cycle (default: daily)'
              end
            end
            SelectContent do
              SelectItem(value: 'daily') { 'Daily' }
              SelectItem(value: 'weekly') { 'Weekly' }
              SelectItem(value: 'monthly') { 'Monthly' }
            end
          end
        end
      end

      def render_notes_field(_f)
        FormField(class: 'md:col-span-2') do
          FormFieldLabel(for: 'prescription_notes') { 'Notes' }
          FormFieldHint { 'Additional instructions or information' }
          Textarea(
            rows: 3,
            name: 'prescription[notes]',
            id: 'prescription_notes',
            placeholder: 'Enter any additional notes or instructions...',
            value: prescription.notes
          )
        end
      end

      def render_actions(_f)
        div(class: 'flex gap-3 justify-end') do
          Link(href: person_path(person), variant: :outline) { 'Cancel' }
          Button(
            type: :submit,
            variant: :primary,
            size: :md,
            data: { prescription_form_target: 'submit' }
          ) { prescription.new_record? ? 'Add Prescription' : 'Update Prescription' }
        end
      end

      def format_dosage_option(dosage)
        "#{dosage.amount.to_f} #{dosage.unit} - #{dosage.description}"
      end
    end
  end
end
