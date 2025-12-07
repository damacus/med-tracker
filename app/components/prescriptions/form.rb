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
        Alert(variant: :destructive, class: 'mb-6') do
          AlertTitle { "#{prescription.errors.count} error(s) prohibited this prescription from being saved:" }
          AlertDescription do
            ul(class: 'list-disc list-inside space-y-1') do
              prescription.errors.full_messages.each do |message|
                li { message }
              end
            end
          end
        end
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
            SelectTrigger(data: { testid: 'dosage-trigger', prescription_form_target: 'dosageTrigger' }) do
              SelectValue(
                placeholder: 'Select a dosage',
                data: { prescription_form_target: 'dosageValue' }
              ) do
                if prescription.dosage
                  "#{prescription.dosage.amount} #{prescription.dosage.unit} - #{prescription.dosage.description}"
                else
                  'Select a dosage'
                end
              end
            end
            SelectContent(data: { prescription_form_target: 'dosageContent' }) do
              (prescription.medicine&.dosages || []).each do |dosage|
                SelectItem(value: dosage.id.to_s) do
                  "#{dosage.amount} #{dosage.unit} - #{dosage.description}"
                end
              end
            end
          end
        end
      end

      def render_frequency_field(f)
        FormField do
          FormFieldLabel(for: 'prescription_frequency') { 'Frequency' }
          render f.text_field(
            :frequency,
            class: 'flex h-9 w-full rounded-md border bg-background px-3 py-1 text-sm shadow-sm ' \
                   'border-border placeholder:text-muted-foreground focus-visible:outline-none ' \
                   'focus-visible:ring-1 focus-visible:ring-ring',
            placeholder: 'e.g., Once daily, Every 4-6 hours',
            data: { action: 'input->prescription-form#validate' }
          )
        end
      end

      def render_start_date_field(f)
        FormField do
          FormFieldLabel(for: 'prescription_start_date') { 'Start date' }
          render f.date_field(
            :start_date,
            class: 'flex h-9 w-full rounded-md border bg-background px-3 py-1 text-sm shadow-sm ' \
                   'border-border focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring',
            data: { action: 'change->prescription-form#validate' }
          )
        end
      end

      def render_end_date_field(f)
        FormField do
          FormFieldLabel(for: 'prescription_end_date') { 'End date' }
          render f.date_field(
            :end_date,
            class: 'flex h-9 w-full rounded-md border bg-background px-3 py-1 text-sm shadow-sm ' \
                   'border-border focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring'
          )
        end
      end

      def render_max_doses_field(f)
        FormField do
          FormFieldLabel(for: 'prescription_max_daily_doses') { 'Max doses per cycle' }
          FormFieldHint { 'Maximum number of doses allowed per cycle' }
          render f.number_field(
            :max_daily_doses,
            min: 1,
            class: 'flex h-9 w-full rounded-md border bg-background px-3 py-1 text-sm shadow-sm ' \
                   'border-border focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring'
          )
        end
      end

      def render_min_hours_field(f)
        FormField do
          FormFieldLabel(for: 'prescription_min_hours_between_doses') { 'Minimum hours between doses' }
          FormFieldHint { 'Minimum time required between doses' }
          render f.number_field(
            :min_hours_between_doses,
            min: 1,
            class: 'flex h-9 w-full rounded-md border bg-background px-3 py-1 text-sm shadow-sm ' \
                   'border-border focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring'
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

      def select_classes
        'flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ' \
          'ring-offset-background focus-visible:outline-none focus-visible:ring-2 ' \
          'focus-visible:ring-ring focus-visible:ring-offset-2'
      end
    end
  end
end
