# frozen_string_literal: true

module Components
  module Schedules
    # Renders a schedule form using RubyUI components
    class Form < Components::Base
      include Phlex::Rails::Helpers::FormWith

      attr_reader :schedule, :person, :medications

      def initialize(schedule:, person:, medications:)
        @schedule = schedule
        @person = person
        @medications = medications
        super()
      end

      def view_template
        form_with(
          model: [person, schedule],
          class: 'space-y-6',
          data: { controller: 'schedule-form', turbo_stream: true }
        ) do |f|
          render_errors if schedule.errors.any?
          render_form_fields(f)
          render_actions(f)
        end
      end

      private

      def render_errors
        Alert(variant: :destructive, class: 'mb-6') do
          AlertTitle { "#{schedule.errors.count} error(s) prohibited this schedule from being saved:" }
          AlertDescription do
            ul(class: 'my-2 ml-6 list-disc [&>li]:mt-1') do
              schedule.errors.full_messages.each do |message|
                li { message }
              end
            end
          end
        end
      end

      def render_form_fields(f)
        div(class: 'grid grid-cols-1 md:grid-cols-2 gap-6') do
          render_medication_field(f)
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

      def render_medication_field(_f)
        FormField do
          FormFieldLabel(for: 'schedule_medication_id') { 'Medication' }
          Select(data: { schedule_form_target: 'medicationSelect', testid: 'medication-select' }) do
            SelectInput(
              name: 'schedule[medication_id]',
              id: 'schedule_medication_id',
              value: schedule.medication_id,
              required: true,
              data: { action: 'change->schedule-form#updateDosages' }
            )
            SelectTrigger(data: { testid: 'medication-trigger' }) do
              SelectValue(placeholder: 'Select a medication') do
                schedule.medication&.name || 'Select a medication'
              end
            end
            SelectContent do
              medications.each do |medication|
                SelectItem(value: medication.id.to_s) { medication.name }
              end
            end
          end
        end
      end

      def render_dosage_field(_f)
        FormField do
          FormFieldLabel(for: 'schedule_dosage_id') { 'Dosage' }
          Select(data: { schedule_form_target: 'dosageSelect', testid: 'dosage-select' }) do
            SelectInput(
              name: 'schedule[dosage_id]',
              id: 'schedule_dosage_id',
              value: schedule.dosage_id,
              required: true,
              data: { action: 'change->schedule-form#validate' }
            )
            SelectTrigger(
              disabled: schedule.medication.nil?,
              aria_disabled: schedule.medication.nil?,
              data: { testid: 'dosage-trigger', schedule_form_target: 'dosageTrigger' }
            ) do
              SelectValue(
                placeholder: 'Select a medication first',
                data: { schedule_form_target: 'dosageValue' }
              ) do
                if schedule.dosage
                  format_dosage_option(schedule.dosage)
                else
                  'Select a medication first'
                end
              end
            end
            SelectContent(data: { schedule_form_target: 'dosageContent' }) do
              (schedule.medication&.dosages || []).each do |dosage|
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
          FormFieldLabel(for: 'schedule_frequency') { 'Frequency' }
          Input(
            type: :text,
            name: 'schedule[frequency]',
            id: 'schedule_frequency',
            value: schedule.frequency,
            placeholder: 'e.g., Once daily, Every 4-6 hours',
            data: { action: 'input->schedule-form#validate' }
          )
        end
      end

      def render_start_date_field(_f)
        FormField do
          FormFieldLabel(for: 'schedule_start_date') { 'Start date' }
          Input(
            type: :string,
            name: 'schedule[start_date]',
            id: 'schedule_start_date',
            value: schedule.start_date&.to_fs(:db),
            required: true,
            placeholder: 'Select a date',
            data: {
              controller: 'ruby-ui--calendar-input',
              action: 'input->schedule-form#validate'
            }
          )
          Calendar(
            input_id: '#schedule_start_date',
            date_format: 'yyyy-MM-dd',
            class: 'rounded-md border shadow'
          )
        end
      end

      def render_end_date_field(_f)
        FormField do
          FormFieldLabel(for: 'schedule_end_date') { 'End date' }
          Input(
            type: :string,
            name: 'schedule[end_date]',
            id: 'schedule_end_date',
            value: schedule.end_date&.to_fs(:db),
            placeholder: 'Select a date',
            data: { controller: 'ruby-ui--calendar-input' }
          )
          Calendar(
            input_id: '#schedule_end_date',
            date_format: 'yyyy-MM-dd',
            class: 'rounded-md border shadow'
          )
        end
      end

      def render_max_doses_field(_f)
        FormField do
          FormFieldLabel(for: 'schedule_max_daily_doses') { 'Max doses per cycle' }
          FormFieldHint { 'Maximum number of doses allowed per cycle' }
          Input(
            type: :number,
            name: 'schedule[max_daily_doses]',
            id: 'schedule_max_daily_doses',
            value: schedule.max_daily_doses,
            min: 1
          )
        end
      end

      def render_min_hours_field(_f)
        FormField do
          FormFieldLabel(for: 'schedule_min_hours_between_doses') { 'Minimum hours between doses' }
          FormFieldHint { 'Minimum time required between doses' }
          Input(
            type: :number,
            name: 'schedule[min_hours_between_doses]',
            id: 'schedule_min_hours_between_doses',
            value: schedule.min_hours_between_doses,
            min: 1
          )
        end
      end

      def render_dose_cycle_field(_f)
        FormField do
          FormFieldLabel(for: 'schedule_dose_cycle') { 'Dose cycle' }
          FormFieldHint { 'How often the dose cycle resets (default: daily)' }
          Select do
            SelectInput(
              name: 'schedule[dose_cycle]',
              id: 'schedule_dose_cycle',
              value: schedule.dose_cycle
            )
            SelectTrigger do
              SelectValue(placeholder: 'Select a cycle (default: daily)') do
                schedule.dose_cycle&.titleize || 'Select a cycle (default: daily)'
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
          FormFieldLabel(for: 'schedule_notes') { 'Notes' }
          FormFieldHint { 'Additional instructions or information' }
          Textarea(
            rows: 3,
            name: 'schedule[notes]',
            id: 'schedule_notes',
            placeholder: 'Enter any additional notes or instructions...',
            value: schedule.notes
          )
        end
      end

      def render_actions(_f)
        div(class: 'flex gap-3 justify-end') do
          Button(variant: :ghost, data: { action: 'click->modal#close' }) { 'Cancel' }
          Button(
            type: :submit,
            variant: :primary,
            size: :md,
            data: { schedule_form_target: 'submit' }
          ) { schedule.new_record? ? 'Add Schedule' : 'Update Schedule' }
        end
      end

      def format_dosage_option(dosage)
        "#{dosage.amount.to_f} #{dosage.unit} - #{dosage.description}"
      end
    end
  end
end
