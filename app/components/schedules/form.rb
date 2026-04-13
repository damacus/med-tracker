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
          data: {
            controller: 'schedule-form',
            turbo_stream: true,
            person_type: person.person_type,
            schedule_form_dose_options_value: medication_dose_options.to_json,
            schedule_form_next_url_value: new_person_schedule_path(person),
            schedule_form_translations_value: {
              selectDosage: t('schedules.form.select_dosage'),
              selectMedicationFirst: t('schedules.form.select_medication_first'),
              frequencyOncePerCycle: t('schedules.form.frequency_once_per_cycle'),
              frequencyUpToPerCycle: t('schedules.form.frequency_up_to_per_cycle'),
              frequencyOnce: t('schedules.form.frequency_once'),
              frequencyUpTo: t('schedules.form.frequency_up_to'),
              frequencyAtLeastHours: t('schedules.form.frequency_at_least_hours')
            }.to_json
          }
        ) do |f|
          render_errors if schedule.errors.any?
          render_dose_snapshot_inputs
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
        if schedule.new_record? && schedule.medication.blank?
          render_medication_step
        elsif schedule.new_record?
          render_details_step
        else
          div(class: 'grid grid-cols-1 md:grid-cols-2 gap-6') do
            render_medication_field(f)
            render_dosage_field(f)
            render_start_date_field(f)
            render_end_date_field(f)
            render_frequency_field(f)
            div(class: 'md:col-span-2 grid grid-cols-1 md:grid-cols-3 gap-6') do
              render_max_doses_field(f)
              render_min_hours_field(f)
              render_dose_cycle_field(f)
            end
            render_notes_field(f)
          end
        end
      end

      def render_medication_step
        div(class: 'space-y-6') do
          div(class: 'space-y-2') do
            Heading(level: 2, size: '4', class: 'font-semibold') { t('schedules.form.choose_medication_title') }
            Text(size: '2', class: 'text-muted-foreground') do
              t('schedules.form.choose_medication_description')
            end
          end
          render_medication_step_field
        end
      end

      def render_details_step
        div(class: 'space-y-6') do
          div(
            class: 'flex items-center justify-between rounded-2xl border ' \
                   'border-border bg-surface-container-low px-4 py-3'
          ) do
            div do
              Text(
                size: '1', weight: 'black',
                class: 'uppercase tracking-widest text-muted-foreground'
              ) { t('schedules.form.medication') }
              Text(size: '3', weight: 'semibold') { schedule.medication.name }
            end
            Link(
              href: new_person_schedule_path(person),
              variant: :ghost, size: :sm,
              data: { turbo_frame: 'modal' }
            ) do
              t('schedules.form.change')
            end
          end
          input(type: :hidden, name: 'schedule[medication_id]', value: schedule.medication_id)
          div(class: 'grid grid-cols-1 md:grid-cols-2 gap-6') do
            render_dosage_cards
            render_details_intro
            render_frequency_field(nil)
            render_start_date_field(nil)
            render_end_date_field(nil)
            div(class: 'md:col-span-2 grid grid-cols-1 md:grid-cols-3 gap-6') do
              render_max_doses_field(nil)
              render_min_hours_field(nil)
              render_dose_cycle_field(nil)
            end
            render_notes_field(nil)
          end
        end
      end

      def render_dose_snapshot_inputs
        input(
          type: :hidden,
          name: 'schedule[dose_amount]',
          value: schedule.dose_amount&.to_s,
          data: { schedule_form_target: 'doseAmountInput' }
        )
        input(
          type: :hidden,
          name: 'schedule[dose_unit]',
          value: schedule.dose_unit,
          data: { schedule_form_target: 'doseUnitInput' }
        )
      end

      def render_medication_field(_f, action: 'change->schedule-form#updateDosages')
        FormField do
          FormFieldLabel(for: 'schedule_medication_id') do
            plain t('schedules.form.medication')
            span(class: 'text-destructive ml-0.5') { ' *' }
          end
          Select(data: { schedule_form_target: 'medicationSelect', testid: 'medication-select' }) do
            SelectInput(
              name: 'schedule[medication_id]',
              id: 'schedule_medication_id',
              value: schedule.medication_id,
              required: true,
              data: { action: action }
            )
            SelectTrigger(data: { testid: 'medication-trigger' }) do
              SelectValue(placeholder: t('schedules.form.select_medication')) do
                schedule.medication&.name || t('schedules.form.select_medication')
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

      def render_medication_step_field
        FormField do
          FormFieldLabel(for: 'schedule_medication_id') do
            plain t('schedules.form.medication')
            span(class: 'text-destructive ml-0.5') { ' *' }
          end
          select(
            name: 'schedule[medication_id]',
            id: 'schedule_medication_id',
            required: true,
            class: 'w-full rounded-md border border-input bg-background px-3 py-2 text-sm',
            data: { action: 'change->schedule-form#advanceToDetails' }
          ) do
            option(value: '') { t('schedules.form.select_medication') }
            medications.each do |medication|
              option(value: medication.id, selected: schedule.medication_id == medication.id) { medication.name }
            end
          end
        end
      end

      def render_dosage_cards
        FormField(class: 'md:col-span-2') do
          FormFieldLabel(for: 'schedule_dose_option_key') do
            plain t('schedules.form.dose')
            span(class: 'text-destructive ml-0.5') { ' *' }
          end
          Text(size: '2', class: 'text-muted-foreground') { t('schedules.form.choose_one_dose') }
          if schedule.medication.dosages.any?
            div(class: 'mt-3 grid grid-cols-1 md:grid-cols-2 gap-3') do
              schedule.medication.dosages.each do |dosage|
                label(class: dosage_card_classes(dosage)) do
                  input(
                    type: :radio,
                    name: 'schedule[dose_option_key]',
                    id: dosage_dom_id(dosage),
                    value: dosage.selection_key,
                    checked: selected_dose_selection_key == dosage.selection_key,
                    required: true,
                    class: 'sr-only',
                    data: {
                      action: 'change->schedule-form#onDosageChange',
                      amount: dosage.amount.to_s,
                      unit: dosage.unit,
                      frequency: dosage.frequency,
                      default_max_daily_doses: dosage.default_max_daily_doses&.to_s,
                      default_min_hours_between_doses: dosage.default_min_hours_between_doses&.to_s,
                      default_dose_cycle: dosage.default_dose_cycle
                    }
                  )
                  div(class: 'font-semibold text-foreground') { "#{dosage.amount.to_f} #{dosage.unit}" }
                  div(class: 'text-sm text-muted-foreground') { dosage.description }
                end
              end
            end
          else
            div(
              class: 'mt-3 rounded-2xl border border-warning bg-warning-container ' \
                     'px-4 py-4 text-sm text-on-warning-container',
              data: { testid: 'schedule-no-dosage-message' }
            ) do
              t('schedules.form.no_dose_options')
            end
          end
        end
      end

      def render_details_intro
        div(class: 'md:col-span-2 space-y-1') do
          Heading(level: 2, size: '4', class: 'font-semibold') { t('schedules.form.schedule_details_title') }
          Text(size: '2', class: 'text-muted-foreground') do
            t('schedules.form.schedule_details_description')
          end
        end
      end

      def render_dosage_field(_f)
        FormField do
          FormFieldLabel(for: 'schedule_dose_option_key') do
            plain t('schedules.form.dosage')
            span(class: 'text-destructive ml-0.5') { ' *' }
          end
          Select(data: { schedule_form_target: 'dosageSelect', testid: 'dosage-select' }) do
            SelectInput(
              name: 'schedule[dose_option_key]',
              id: 'schedule_dose_option_key',
              value: selected_dose_selection_key,
              required: true,
              data: { action: 'change->schedule-form#onDosageChange' }
            )
            SelectTrigger(
              disabled: schedule.medication.nil?,
              aria_disabled: schedule.medication.nil?,
              data: { testid: 'dosage-trigger', schedule_form_target: 'dosageTrigger' }
            ) do
              SelectValue(
                placeholder: t('schedules.form.select_medication_first'),
                data: { schedule_form_target: 'dosageValue' }
              ) do
                if selected_dosage_option
                  format_dosage_option(selected_dosage_option)
                else
                  t('schedules.form.select_medication_first')
                end
              end
            end
            SelectContent(data: { schedule_form_target: 'dosageContent' }) do
              (schedule.medication&.dosages || []).each do |dosage|
                SelectItem(value: dosage.selection_key) do
                  format_dosage_option(dosage)
                end
              end
            end
          end
        end
      end

      def render_frequency_field(_f)
        FormField(class: 'md:col-span-2') do
          FormFieldLabel(for: 'schedule_frequency') { t('schedules.form.frequency') }
          Input(
            type: :text,
            name: 'schedule[frequency]',
            id: 'schedule_frequency',
            value: schedule.frequency,
            placeholder: t('schedules.form.frequency_placeholder'),
            data: { action: 'input->schedule-form#validate', schedule_form_target: 'frequencyInput' }
          )
        end
      end

      def render_start_date_field(_f)
        FormField do
          FormFieldLabel(for: 'schedule_start_date') do
            plain t('schedules.form.start_date')
            span(class: 'text-destructive ml-0.5') { ' *' }
          end
          Input(
            type: :string,
            name: 'schedule[start_date]',
            id: 'schedule_start_date',
            value: schedule.start_date&.to_fs(:db),
            required: true,
            placeholder: t('schedules.form.select_date'),
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
          FormFieldLabel(for: 'schedule_end_date') { t('schedules.form.end_date') }
          Input(
            type: :string,
            name: 'schedule[end_date]',
            id: 'schedule_end_date',
            value: schedule.end_date&.to_fs(:db),
            placeholder: t('schedules.form.select_date'),
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
          FormFieldLabel(for: 'schedule_max_daily_doses') { t('schedules.form.max_doses_per_cycle') }
          FormFieldHint { t('schedules.form.max_doses_hint') }
          Input(
            type: :number,
            name: 'schedule[max_daily_doses]',
            id: 'schedule_max_daily_doses',
            value: schedule.max_daily_doses,
            placeholder: t('schedules.form.max_doses_placeholder'),
            min: 1,
            data: { schedule_form_target: 'maxDosesInput', action: 'input->schedule-form#generateFrequency' }
          )
        end
      end

      def render_min_hours_field(_f)
        FormField do
          FormFieldLabel(for: 'schedule_min_hours_between_doses') { t('schedules.form.min_hours_between_doses') }
          FormFieldHint { t('schedules.form.min_hours_hint') }
          Input(
            type: :number,
            name: 'schedule[min_hours_between_doses]',
            id: 'schedule_min_hours_between_doses',
            value: schedule.min_hours_between_doses,
            placeholder: t('schedules.form.min_hours_placeholder'),
            min: 0,
            step: '0.5',
            data: { schedule_form_target: 'minHoursInput', action: 'input->schedule-form#generateFrequency' }
          )
        end
      end

      def render_dose_cycle_field(_f)
        FormField do
          FormFieldLabel(for: 'schedule_dose_cycle') { t('schedules.form.dose_cycle') }
          FormFieldHint { t('schedules.form.dose_cycle_hint') }
          Select do
            SelectInput(
              name: 'schedule[dose_cycle]',
              id: 'schedule_dose_cycle',
              value: schedule.dose_cycle,
              data: { schedule_form_target: 'doseCycleInput', action: 'change->schedule-form#generateFrequency' }
            )
            SelectTrigger do
              SelectValue(placeholder: t('schedules.form.select_cycle')) do
                schedule.dose_cycle&.titleize || t('schedules.form.select_cycle')
              end
            end
            SelectContent do
              Schedule::DOSE_CYCLE_OPTIONS.each do |label, value|
                SelectItem(value: value) { label }
              end
            end
          end
        end
      end

      def render_notes_field(_f)
        FormField(class: 'md:col-span-2') do
          FormFieldLabel(for: 'schedule_notes') { t('schedules.form.notes') }
          FormFieldHint { t('schedules.form.notes_hint') }
          Textarea(
            rows: 3,
            name: 'schedule[notes]',
            id: 'schedule_notes',
            placeholder: t('schedules.form.notes_placeholder'),
            value: schedule.notes
          )
        end
      end

      def render_actions(_f)
        div(class: 'flex gap-3 justify-end') do
          Button(variant: :ghost, data: { action: 'click->ruby-ui--dialog#dismiss' }) { t('schedules.form.cancel') }
          unless schedule.new_record? && schedule.medication.blank?
            Button(
              type: :submit,
              variant: :primary,
              size: :md,
              disabled: schedule.new_record? && (schedule.dose_amount.blank? || schedule.dose_unit.blank?),
              data: { schedule_form_target: 'submit' }
            ) { schedule.new_record? ? t('schedules.form.add_plan') : t('schedules.form.update_plan') }
          end
        end
      end

      def format_dosage_option(dosage)
        "#{dosage.amount.to_f} #{dosage.unit} - #{dosage.description}"
      end

      def dosage_card_classes(dosage)
        base = 'rounded-2xl border p-4 transition-colors cursor-pointer'
        selected = if selected_dose_selection_key == dosage.selection_key
                     ' border-primary bg-primary/5 ring-2 ring-primary/20'
                   else
                     ' border-border bg-surface-container-lowest'
                   end
        "#{base}#{selected}"
      end

      def selected_dosage_option
        return @selected_dosage_option if defined?(@selected_dosage_option)

        @selected_dosage_option = (schedule.medication&.dosages || []).find do |dosage|
          dosage.amount.to_s == schedule.dose_amount.to_s && dosage.unit == schedule.dose_unit
        end
      end

      def selected_dose_selection_key
        selected_dosage_option&.selection_key
      end

      def dosage_dom_id(dosage)
        "schedule_dose_option_#{dosage.selection_key.parameterize(separator: '_')}"
      end

      def medication_dose_options
        medications.each_with_object({}) do |medication, dose_options|
          dose_options[medication.id.to_s] = medication.dose_options_payload
        end
      end
    end
  end
end
