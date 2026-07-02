# frozen_string_literal: true

module Components
  module Schedules
    class Fields < Components::Base
      attr_reader :schedule, :person, :medications

      def initialize(schedule:, person:, medications:)
        @schedule = schedule
        @person = person
        @medications = medications
        super()
      end

      def view_template
        if schedule.new_record? && schedule.medication.blank?
          render_medication_step
        elsif schedule.new_record?
          render_details_step
        else
          render_edit_fields
        end
      end

      private

      def render_edit_fields
        div(class: 'grid grid-cols-1 md:grid-cols-2 gap-6') do
          render_medication_field
          render_dosage_field
          render_start_date_field
          render_end_date_field
          render_frequency_field
          render_timing_fields
          render_frequency_preview
          render_notes_field
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
          render_selected_medication_summary
          input(type: :hidden, name: 'schedule[medication_id]', value: schedule.medication_id)
          div(class: 'grid grid-cols-1 md:grid-cols-2 gap-6') do
            render_dosage_cards
            render_details_intro
            render_frequency_field
            render_start_date_field
            render_end_date_field
            render_timing_fields
            render_frequency_preview
            render_notes_field
          end
        end
      end

      def render_selected_medication_summary
        div(
          class: 'flex items-center justify-between rounded-2xl border ' \
                 'border-outline-variant/30 bg-surface-container-low px-4 py-3'
        ) do
          div do
            m3_text(
              variant: :label_small,
              class: 'uppercase tracking-widest text-on-surface-variant font-black'
            ) { t('schedules.form.medication') }
            m3_text(variant: :title_medium, class: 'font-bold') { schedule.medication.display_name }
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
      end

      def render_timing_fields
        div(class: 'md:col-span-2 grid grid-cols-1 md:grid-cols-3 gap-6') do
          render_max_doses_field
          render_min_hours_field
          render_dose_cycle_field
        end
      end

      def render_medication_field(action: 'change->schedule-form#updateDosages')
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
                SelectItem(value: medication.id.to_s) { medication.display_name }
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
              class: 'rounded-shape-sm border-outline-variant bg-surface-container-lowest py-4 px-4 transition-all'
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
                    span { medication.display_name }
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
          if dosage_options.dosages.any?
            div(class: 'mt-3 grid grid-cols-1 md:grid-cols-2 gap-3') do
              dosage_options.dosages.each do |dosage|
                render_dosage_card(dosage)
              end
            end
          else
            render_no_dosage_message
          end
        end
      end

      def render_dosage_card(dosage)
        label(class: dosage_card_classes(dosage)) do
          input(
            type: :radio,
            name: 'schedule[dose_option_key]',
            id: dosage_options.dosage_dom_id(dosage),
            value: dosage.option_value,
            checked: dosage_options.selected_dose_option_value == dosage.option_value,
            required: true,
            class: 'sr-only',
            data: {
              action: 'change->schedule-form#onDosageChange',
              id: dosage.id,
              amount: dosage.amount.to_s,
              unit: dosage.unit,
              frequency: dosage.frequency,
              default_max_daily_doses: dosage.default_max_daily_doses&.to_s,
              default_min_hours_between_doses: dosage.default_min_hours_between_doses&.to_s,
              default_dose_cycle: dosage.default_dose_cycle
            }
          )
          div(class: 'font-bold text-foreground') { DoseAmount.new(dosage.amount, dosage.unit).to_s }
          div(class: 'text-sm text-on-surface-variant font-medium') { dosage.description }
        end
      end

      def render_no_dosage_message
        div(
          class: 'mt-3 rounded-2xl border border-outline-variant/30 bg-secondary-container/20 ' \
                 'px-4 py-4 text-sm text-on-surface-variant font-medium',
          data: { testid: 'schedule-no-dosage-message' }
        ) do
          t('schedules.form.no_dose_options')
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

      def render_dosage_field
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
              value: dosage_options.selected_dose_option_value,
              required: true,
              data: { action: 'change->schedule-form#onDosageChange' }
            )
            SelectTrigger(
              disabled: schedule.medication.nil?,
              aria_disabled: schedule.medication.nil?,
              class: 'rounded-shape-sm border-outline-variant bg-surface-container-lowest',
              data: { testid: 'dosage-trigger', schedule_form_target: 'dosageTrigger' }
            ) do
              SelectValue(
                placeholder: t('schedules.form.select_medication_first'),
                data: { schedule_form_target: 'dosageValue' }
              ) do
                if dosage_options.selected_dosage_option
                  dosage_options.format_dosage_option(dosage_options.selected_dosage_option)
                else
                  t('schedules.form.select_medication_first')
                end
              end
            end
            SelectContent(data: { schedule_form_target: 'dosageContent' }) do
              dosage_options.dosages.each do |dosage|
                SelectItem(value: dosage.option_value) do
                  dosage_options.format_dosage_option(dosage)
                end
              end
            end
          end
        end
      end

      def render_frequency_field
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
            class: 'rounded-shape-sm border-outline-variant bg-surface-container-lowest py-4 px-4 transition-all',
            data: {
              action: 'input->schedule-form#validate change->schedule-form#validate',
              schedule_form_target: 'frequencyInput'
            }
          )
        end
      end

      def render_start_date_field
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
            class: 'rounded-shape-sm border-outline-variant bg-surface-container-lowest py-4 px-4 transition-all',
            data: {
              controller: 'ruby-ui--calendar-input',
              action: 'input->schedule-form#validate change->schedule-form#validate'
            }
          )
          Calendar(
            input_id: '#schedule_start_date',
            date_format: 'yyyy-MM-dd',
            class: 'rounded-shape-sm border shadow-elevation-2 bg-surface-container-high'
          )
        end
      end

      def render_end_date_field
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
            class: 'rounded-shape-sm border-outline-variant bg-surface-container-lowest py-4 px-4 transition-all',
            data: { controller: 'ruby-ui--calendar-input' }
          )
          Calendar(
            input_id: '#schedule_end_date',
            date_format: 'yyyy-MM-dd',
            class: 'rounded-shape-sm border shadow-elevation-2 bg-surface-container-high'
          )
        end
      end

      def render_max_doses_field
        FormField do
          render RubyUI::FormFieldLabel.new(
            for: 'schedule_max_daily_doses',
            class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
          ) { t('schedules.form.max_doses_per_cycle') }
          render_field_hint { t('schedules.form.max_doses_hint') }
          m3_input(
            type: :number,
            name: 'schedule[max_daily_doses]',
            id: 'schedule_max_daily_doses',
            value: schedule.max_daily_doses,
            placeholder: t('schedules.form.max_doses_placeholder'),
            min: 1,
            class: 'rounded-shape-sm border-outline-variant bg-surface-container-lowest py-4 px-4 transition-all',
            data: { schedule_form_target: 'maxDosesInput', action: 'input->schedule-form#generateFrequency' }
          )
        end
      end

      def render_min_hours_field
        FormField do
          render RubyUI::FormFieldLabel.new(
            for: 'schedule_min_hours_between_doses',
            class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
          ) { t('schedules.form.min_hours_between_doses') }
          render_field_hint { t('schedules.form.min_hours_hint') }
          m3_input(
            type: :number,
            name: 'schedule[min_hours_between_doses]',
            id: 'schedule_min_hours_between_doses',
            value: schedule.min_hours_between_doses,
            placeholder: t('schedules.form.min_hours_placeholder'),
            min: 0,
            step: '0.5',
            class: 'rounded-shape-sm border-outline-variant bg-surface-container-lowest py-4 px-4 transition-all',
            data: { schedule_form_target: 'minHoursInput', action: 'input->schedule-form#generateFrequency' }
          )
        end
      end

      def render_dose_cycle_field
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'schedule_dose_cycle_trigger',
            class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
          ) { t('schedules.form.dose_cycle') }
          render_field_hint { t('schedules.form.dose_cycle_hint') }
          render RubyUI::Combobox.new(class: 'w-full') do
            render RubyUI::ComboboxTrigger.new(
              placeholder: schedule.dose_cycle&.titleize || t('schedules.form.select_cycle'),
              class: 'rounded-shape-sm border-outline-variant bg-surface-container-lowest py-4 px-4 transition-all'
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

      def render_notes_field
        FormField(class: 'md:col-span-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'schedule_notes',
            class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
          ) { t('schedules.form.notes') }
          render_field_hint { t('schedules.form.notes_hint') }
          render RubyUI::Textarea.new(
            rows: 3,
            name: 'schedule[notes]',
            id: 'schedule_notes',
            placeholder: t('schedules.form.notes_placeholder'),
            value: schedule.notes,
            class: 'rounded-shape-sm border-outline-variant bg-surface-container-lowest p-4 ' \
                   'focus:ring-2 focus:ring-primary/10 ' \
                   'focus:border-primary transition-all resize-none'
          )
        end
      end

      def render_frequency_preview
        div(class: 'md:col-span-2') do
          render Components::Schedules::FrequencyPreview.new(
            max_daily_doses: schedule.max_daily_doses,
            min_hours_between_doses: schedule.min_hours_between_doses,
            dose_cycle: schedule.dose_cycle
          )
        end
      end

      def render_field_hint(&)
        render Components::Shared::FieldHint.new, &
      end

      def dosage_card_classes(dosage)
        base = 'rounded-2xl border p-4 transition-colors cursor-pointer'
        selected = if dosage_options.selected_dose_option_value == dosage.option_value
                     ' border-primary bg-primary/5 ring-2 ring-primary/20'
                   else
                     ' border-border bg-card'
                   end
        "#{base}#{selected}"
      end

      def dosage_options
        @dosage_options ||= ::Schedules::DosageOptionsPresenter.new(schedule: schedule)
      end
    end
  end
end
