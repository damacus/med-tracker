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

      def render_medicine_field(f)
        FormField do
          FormFieldLabel(for: 'prescription_medicine_id') { 'Medicine' }
          render f.collection_select(
            :medicine_id,
            medicines,
            :id,
            :name,
            { prompt: 'Select a medicine' },
            {
              class: 'flex h-9 w-full rounded-md border bg-background px-3 py-1 text-sm shadow-sm ' \
                     'border-border focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring',
              data: { action: 'change->prescription-form#updateDosages' }
            }
          )
        end
      end

      def render_dosage_field(f)
        FormField do
          FormFieldLabel(for: 'prescription_dosage_id') { 'Dosage' }
          render f.collection_select(
            :dosage_id,
            prescription.medicine&.dosages || [],
            :id,
            ->(d) { "#{d.amount} #{d.unit} - #{d.description}" },
            { prompt: 'Select a dosage' },
            {
              class: 'flex h-9 w-full rounded-md border bg-background px-3 py-1 text-sm shadow-sm ' \
                     'border-border focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring',
              data: { prescription_form_target: 'dosageSelect' }
            }
          )
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
            placeholder: 'e.g., Once daily, Every 4-6 hours'
          )
        end
      end

      def render_start_date_field(f)
        FormField do
          FormFieldLabel(for: 'prescription_start_date') { 'Start date' }
          render f.date_field(
            :start_date,
            class: 'flex h-9 w-full rounded-md border bg-background px-3 py-1 text-sm shadow-sm ' \
                   'border-border focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring'
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

      def render_dose_cycle_field(f)
        FormField do
          FormFieldLabel(for: 'prescription_dose_cycle') { 'Dose cycle' }
          FormFieldHint { 'How often the dose cycle resets (default: daily)' }
          render f.select(
            :dose_cycle,
            [['Daily', 'daily'], ['Weekly', 'weekly'], ['Monthly', 'monthly']],
            { include_blank: 'Select a cycle (default: daily)' },
            {
              class: 'flex h-9 w-full rounded-md border bg-background px-3 py-1 text-sm shadow-sm ' \
                     'border-border focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring'
            }
          )
        end
      end

      def render_notes_field(f)
        FormField(class: 'md:col-span-2') do
          FormFieldLabel(for: 'prescription_notes') { 'Notes' }
          FormFieldHint { 'Additional instructions or information' }
          render f.text_area(
            :notes,
            rows: 3,
            class: 'flex min-h-[60px] w-full rounded-md border bg-background px-3 py-2 text-sm shadow-sm ' \
                   'border-border placeholder:text-muted-foreground focus-visible:outline-none ' \
                   'focus-visible:ring-1 focus-visible:ring-ring'
          )
        end
      end

      def render_actions(f)
        div(class: 'flex gap-3 justify-end') do
          Link(href: person_path(person), variant: :outline) { 'Cancel' }
          render f.submit(
            prescription.new_record? ? 'Add Prescription' : 'Update Prescription',
            class: 'inline-flex items-center justify-center rounded-md text-sm font-medium ' \
                   'px-4 py-2 h-9 bg-primary text-primary-foreground shadow hover:bg-primary/90 ' \
                   'transition-colors',
            data: { prescription_form_target: 'submit' }
          )
        end
      end
    end
  end
end
