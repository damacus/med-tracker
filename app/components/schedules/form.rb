# frozen_string_literal: true

module Components
  module Schedules
    # Renders a schedule form using RubyUI components
    class Form < Components::Base
      include Phlex::Rails::Helpers::FormWith

      attr_reader :schedule, :person, :medications, :frame_id

      def initialize(schedule:, person:, medications:, frame_id: nil)
        @schedule = schedule
        @person = person
        @medications = medications
        @frame_id = frame_id
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
            schedule_form_frame_id_value: frame_id,
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
        render RubyUI::Alert.new(variant: :destructive,
                                 class: 'mb-8 rounded-shape-xl border-none shadow-elevation-1') do
          div(class: 'flex items-start gap-3') do
            render Icons::AlertCircle.new(size: 20)
            div do
              m3_heading(variant: :title_medium, level: 2, class: 'font-bold mb-1') do
                plain "#{schedule.errors.count} error(s) prohibited this schedule from being saved:"
              end
              ul(class: 'text-sm opacity-90 list-disc pl-4 space-y-1 font-medium') do
                schedule.errors.full_messages.each do |message|
                  li { message }
                end
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
            m3_heading(variant: :title_medium, level: 2, class: 'font-semibold') do
              t('schedules.form.choose_medication_title')
            end
            m3_text(variant: :body_medium, class: 'text-on-surface-variant font-medium') do
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
                   'border-outline-variant/30 bg-surface-container-low px-4 py-3'
          ) do
            div do
              m3_text(
                variant: :label_small,
                class: 'uppercase tracking-widest text-on-surface-variant font-black'
              ) { t('schedules.form.medication') }
              m3_text(variant: :title_medium, class: 'font-bold') { schedule.medication.name }
            end
            m3_link(
              href: new_person_schedule_path(person),
              variant: :text, size: :sm,
              class: 'font-bold',
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
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'schedule_medication_id_trigger',
            class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
          ) do
            plain t('schedules.form.medication')
            span(class: 'text-error ml-0.5') { ' *' }
          end
          render RubyUI::Combobox.new(class: 'w-full') do
            render RubyUI::ComboboxTrigger.new(
              id: 'medication_trigger',
              placeholder: schedule.medication&.name || t('schedules.form.select_medication'),
              class: 'rounded-md border-outline-variant bg-surface-container-lowest py-4 px-4 transition-all'
            )

            render RubyUI::ComboboxPopover.new do
              render RubyUI::ComboboxSearchInput.new(
                placeholder: t('schedules.form.select_medication')
              )

              render RubyUI::ComboboxList.new do
                render(RubyUI::ComboboxEmptyState.new { t('schedules.form.select_medication') })

                medications.each do |medication|
                  render RubyUI::ComboboxItem.new do
                    render RubyUI::ComboboxRadio.new(
                      name: 'schedule[medication_id]',
                      id: "schedule_medication_id_#{medication.id}",
                      value: medication.id,
                      checked: schedule.medication_id == medication.id,
                      required: true,
                      data: { action: 'change->schedule-form#advanceToDetails' }
                    )
                    span { medication.name }
                  end
                end
              end
            end
          end
        end
      end

      def render_dosage_cards
        FormField(class: 'md:col-span-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'schedule_dose_option_key',
            class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
          ) do
            plain t('schedules.form.dose')
            span(class: 'text-error ml-0.5') { ' *' }
          end
          m3_text(variant: :body_medium, class: 'text-on-surface-variant font-medium') do
            t('schedules.form.choose_one_dose')
          end
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
                  div(class: 'font-bold text-foreground') { "#{dosage.amount.to_f} #{dosage.unit}" }
                  div(class: 'text-sm text-on-surface-variant font-medium') { dosage.description }
                end
              end
            end
          else
            div(
              class: 'mt-3 rounded-2xl border border-outline-variant/30 bg-secondary-container/20 ' \
                     'px-4 py-4 text-sm text-on-surface-variant font-medium',
              data: { testid: 'schedule-no-dosage-message' }
            ) do
              t('schedules.form.no_dose_options')
            end
          end
        end
      end

      def render_details_intro
        div(class: 'md:col-span-2 space-y-1') do
          m3_heading(variant: :title_medium, level: 2, class: 'font-semibold') do
            t('schedules.form.schedule_details_title')
          end
          m3_text(variant: :body_medium, class: 'text-on-surface-variant font-medium') do
            t('schedules.form.schedule_details_description')
          end
        end
      end

      def render_dosage_field(_f)
        FormField do
          render RubyUI::FormFieldLabel.new(
            for: 'schedule_dose_option_key',
            class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
          ) do
            plain t('schedules.form.dosage')
            span(class: 'text-error ml-0.5') { ' *' }
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
              class: 'rounded-md border-outline-variant bg-surface-container-lowest',
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
          render RubyUI::FormFieldLabel.new(
            for: 'schedule_frequency',
            class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
          ) { t('schedules.form.frequency') }
          m3_input(
            type: :text,
            name: 'schedule[frequency]',
            id: 'schedule_frequency',
            value: schedule.frequency,
            placeholder: t('schedules.form.frequency_placeholder'),
            class: 'rounded-md border-outline-variant bg-surface-container-lowest py-4 px-4 transition-all',
            data: {
              action: 'input->schedule-form#validate change->schedule-form#validate',
              schedule_form_target: 'frequencyInput'
            }
          )
        end
      end

      def render_start_date_field(_f)
        FormField do
          render RubyUI::FormFieldLabel.new(
            for: 'schedule_start_date',
            class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
          ) do
            plain t('schedules.form.start_date')
            span(class: 'text-error ml-0.5') { ' *' }
          end
          m3_input(
            type: :string,
            name: 'schedule[start_date]',
            id: 'schedule_start_date',
            value: schedule.start_date&.to_fs(:db),
            required: true,
            placeholder: t('schedules.form.select_date'),
            class: 'rounded-md border-outline-variant bg-surface-container-lowest py-4 px-4 transition-all',
            data: {
              controller: 'ruby-ui--calendar-input',
              action: 'input->schedule-form#validate change->schedule-form#validate'
            }
          )
          Calendar(
            input_id: '#schedule_start_date',
            date_format: 'yyyy-MM-dd',
            class: 'rounded-md border shadow-elevation-2 bg-surface-container-high'
          )
        end
      end

      def render_end_date_field(_f)
        FormField do
          render RubyUI::FormFieldLabel.new(
            for: 'schedule_end_date',
            class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
          ) { t('schedules.form.end_date') }
          m3_input(
            type: :string,
            name: 'schedule[end_date]',
            id: 'schedule_end_date',
            value: schedule.end_date&.to_fs(:db),
            placeholder: t('schedules.form.select_date'),
            class: 'rounded-md border-outline-variant bg-surface-container-lowest py-4 px-4 transition-all',
            data: { controller: 'ruby-ui--calendar-input' }
          )
          Calendar(
            input_id: '#schedule_end_date',
            date_format: 'yyyy-MM-dd',
            class: 'rounded-md border shadow-elevation-2 bg-surface-container-high'
          )
        end
      end

      def render_max_doses_field(_f)
        FormField do
          render RubyUI::FormFieldLabel.new(
            for: 'schedule_max_daily_doses',
            class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
          ) { t('schedules.form.max_doses_per_cycle') }
          render RubyUI::FormFieldHint.new(
            class: 'text-xs text-on-surface-variant font-medium ml-1 mb-1'
          ) { t('schedules.form.max_doses_hint') }
          m3_input(
            type: :number,
            name: 'schedule[max_daily_doses]',
            id: 'schedule_max_daily_doses',
            value: schedule.max_daily_doses,
            placeholder: t('schedules.form.max_doses_placeholder'),
            min: 1,
            class: 'rounded-md border-outline-variant bg-surface-container-lowest py-4 px-4 transition-all',
            data: { schedule_form_target: 'maxDosesInput', action: 'input->schedule-form#generateFrequency' }
          )
        end
      end

      def render_min_hours_field(_f)
        FormField do
          render RubyUI::FormFieldLabel.new(
            for: 'schedule_min_hours_between_doses',
            class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
          ) { t('schedules.form.min_hours_between_doses') }
          render RubyUI::FormFieldHint.new(
            class: 'text-xs text-on-surface-variant font-medium ml-1 mb-1'
          ) { t('schedules.form.min_hours_hint') }
          m3_input(
            type: :number,
            name: 'schedule[min_hours_between_doses]',
            id: 'schedule_min_hours_between_doses',
            value: schedule.min_hours_between_doses,
            placeholder: t('schedules.form.min_hours_placeholder'),
            min: 0,
            step: '0.5',
            class: 'rounded-md border-outline-variant bg-surface-container-lowest py-4 px-4 transition-all',
            data: { schedule_form_target: 'minHoursInput', action: 'input->schedule-form#generateFrequency' }
          )
        end
      end

      def render_dose_cycle_field(_f)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'schedule_dose_cycle_trigger',
            class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
          ) { t('schedules.form.dose_cycle') }
          render RubyUI::FormFieldHint.new(
            class: 'text-xs text-on-surface-variant font-medium ml-1 mb-1'
          ) { t('schedules.form.dose_cycle_hint') }
          render RubyUI::Combobox.new(class: 'w-full') do
            render RubyUI::ComboboxTrigger.new(
              placeholder: schedule.dose_cycle&.titleize || t('schedules.form.select_cycle'),
              class: 'rounded-md border-outline-variant bg-surface-container-lowest py-4 px-4 transition-all'
            )

            render RubyUI::ComboboxPopover.new do
              render RubyUI::ComboboxList.new do
                Schedule::DOSE_CYCLE_OPTIONS.each do |label, value|
                  render RubyUI::ComboboxItem.new do
                    render RubyUI::ComboboxRadio.new(
                      name: 'schedule[dose_cycle]',
                      id: "schedule_dose_cycle_#{value}",
                      value: value,
                      checked: schedule.dose_cycle == value,
                      data: { action: 'change->schedule-form#generateFrequency' }
                    )
                    span { label }
                  end
                end
              end
            end
          end
        end
      end

      def render_notes_field(_f)
        FormField(class: 'md:col-span-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'schedule_notes',
            class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
          ) { t('schedules.form.notes') }
          render RubyUI::FormFieldHint.new(
            class: 'text-xs text-on-surface-variant font-medium ml-1 mb-1'
          ) { t('schedules.form.notes_hint') }
          render RubyUI::Textarea.new(
            rows: 3,
            name: 'schedule[notes]',
            id: 'schedule_notes',
            placeholder: t('schedules.form.notes_placeholder'),
            value: schedule.notes,
            class: 'rounded-md border-outline-variant bg-surface-container-lowest p-4 ' \
                   'focus:ring-2 focus:ring-primary/10 ' \
                   'focus:border-primary transition-all resize-none'
          )
        end
      end

      def render_actions(_f)
        div(class: 'flex gap-3 justify-end pt-4') do
          m3_button(variant: :text, size: :lg, class: 'font-bold',
                    data: { action: 'click->ruby-ui--dialog#dismiss' }) do
            t('schedules.form.cancel')
          end
          unless schedule.new_record? && schedule.medication.blank?
            m3_button(
              type: :submit,
              variant: :filled,
              size: :lg,
              class: 'px-8 rounded-shape-xl shadow-lg shadow-primary/20',
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
                     ' border-border bg-card'
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
        key = dosage.selection_key.parameterize(separator: '_')
        return "schedule_dose_option_#{key}" unless duplicate_dose_selection_keys.include?(dosage.selection_key)

        description = dosage.description.to_s.parameterize(separator: '_').presence || dosage.object_id
        "schedule_dose_option_#{key}_#{description}"
      end

      def duplicate_dose_selection_keys
        @duplicate_dose_selection_keys ||= begin
          dosages = schedule.medication&.dosages || []
          grouped_dosages = dosages.group_by(&:selection_key)

          grouped_dosages.each_with_object([]) do |(selection_key, matching_dosages), selection_keys|
            selection_keys << selection_key if matching_dosages.size > 1
          end
        end
      end

      def medication_dose_options
        medications.each_with_object({}) do |medication, dose_options|
          dose_options[medication.id.to_s] = medication.dose_options_payload
        end
      end
    end
  end
end
