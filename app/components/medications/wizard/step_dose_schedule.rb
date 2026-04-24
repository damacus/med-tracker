# frozen_string_literal: true

module Components
  module Medications
    module Wizard
      class StepDoseSchedule < Components::Base
        include FieldHelpers

        SCHEDULE_TYPES = %w[multiple_daily daily weekly specific_dates prn tapering].freeze

        attr_reader :medication, :people

        def initialize(medication:, people:)
          @medication = medication
          @people = people
          super()
        end

        def view_template
          div(
            class: 'space-y-8',
            data: {
              controller: 'medication-schedule-wizard',
              'medication-schedule-wizard-schedule-type-value': default_schedule_type
            }
          ) do
            render_heading
            render_hidden_submission_fields
            render_person_selection
            render_dose_fields
            render_schedule_type_cards
            render_follow_up_fields
            render_date_fields
            render_review_panel
            render_suggested_dosage_records_section
          end
        end

        private

        def render_heading
          div(class: 'space-y-1 mb-2') do
            m3_heading(level: 3, size: '5', class: 'font-bold tracking-tight text-foreground') do
              t('forms.medications.wizard.dose.title')
            end
            m3_text(size: '2', class: 'text-on-surface-variant') do
              t('forms.medications.wizard.dose.description')
            end
          end
        end

        def render_hidden_submission_fields
          render_hidden_primary_dosage_fields
          input(
            type: 'hidden',
            name: 'onboarding_schedule[person_id]',
            value: selected_person_id,
            data: { 'medication-schedule-wizard-target': 'personIdField' }
          )
          input(
            type: 'hidden',
            name: 'onboarding_schedule[schedule_type]',
            value: default_schedule_type,
            data: { 'medication-schedule-wizard-target': 'scheduleTypeField' }
          )
          input(
            type: 'hidden',
            name: 'onboarding_schedule[frequency]',
            value: primary_dosage_value(:frequency),
            data: { 'medication-schedule-wizard-target': 'frequencyField' }
          )
          input(
            type: 'hidden',
            name: 'onboarding_schedule[start_date]',
            value: Time.zone.today.to_s,
            data: { 'medication-schedule-wizard-target': 'startDateField' }
          )
          input(
            type: 'hidden',
            name: 'onboarding_schedule[end_date]',
            value: 1.month.from_now.to_date.to_s,
            data: { 'medication-schedule-wizard-target': 'endDateField' }
          )
          input(
            type: 'hidden',
            name: 'onboarding_schedule[max_daily_doses]',
            value: serialize_hidden_dosage_value(primary_dosage_value(:default_max_daily_doses)),
            data: { 'medication-schedule-wizard-target': 'maxDailyDosesField' }
          )
          input(
            type: 'hidden',
            name: 'onboarding_schedule[min_hours_between_doses]',
            value: serialize_hidden_dosage_value(primary_dosage_value(:default_min_hours_between_doses)),
            data: { 'medication-schedule-wizard-target': 'minHoursBetweenDosesField' }
          )
          input(
            type: 'hidden',
            name: 'onboarding_schedule[dose_cycle]',
            value: primary_dosage_value(:default_dose_cycle) || 'daily',
            data: { 'medication-schedule-wizard-target': 'doseCycleField' }
          )
          input(
            type: 'hidden',
            name: 'onboarding_schedule[schedule_config]',
            value: '{}',
            data: { 'medication-schedule-wizard-target': 'scheduleConfigField' }
          )
        end

        def render_hidden_primary_dosage_fields
          dosage = primary_dosage_record_for_wizard
          index = 0

          input(type: 'hidden', name: dosage_field_name(index, 'id'), value: dosage.id) if dosage&.persisted?
          hidden_primary_dosage_field(index, 'description', dosage&.description)
          hidden_schedule_dosage_field(index, 'amount', dosage&.amount, 'amountField')
          hidden_schedule_dosage_field(index, 'unit', dosage&.unit, 'unitField')
          hidden_schedule_dosage_field(index, 'frequency', dosage&.frequency, 'dosageFrequencyField')
          hidden_schedule_dosage_field(
            index,
            'default_max_daily_doses',
            dosage&.default_max_daily_doses,
            'defaultMaxDailyDosesField'
          )
          hidden_schedule_dosage_field(
            index,
            'default_min_hours_between_doses',
            dosage&.default_min_hours_between_doses,
            'defaultMinHoursBetweenDosesField'
          )
          hidden_schedule_dosage_field(
            index,
            'default_dose_cycle',
            dosage&.default_dose_cycle || 'daily',
            'defaultDoseCycleField'
          )
          hidden_schedule_dosage_field(index, 'default_for_adults', default_for_adults_value, 'defaultForAdultsField')
          hidden_schedule_dosage_field(
            index,
            'default_for_children',
            default_for_children_value,
            'defaultForChildrenField'
          )
        end

        def hidden_schedule_dosage_field(index, field, value, target)
          input(
            type: 'hidden',
            name: dosage_field_name(index, field),
            value: serialize_hidden_dosage_value(value),
            data: { 'medication-schedule-wizard-target': target }
          )
        end

        def render_person_selection
          section(class: 'space-y-3') do
            m3_heading(level: 4, size: '4', class: 'font-bold tracking-tight text-foreground') do
              t('forms.medications.wizard.dose.person_title')
            end
            div(class: 'grid grid-cols-1 sm:grid-cols-2 gap-3') do
              selectable_people.each do |person|
                render_person_card(person)
              end
            end
          end
        end

        def render_person_card(person)
          selected = person == selected_person
          button(
            type: 'button',
            class: selection_card_classes(selected),
            data: {
              action: 'click->medication-schedule-wizard#selectPerson',
              'medication-schedule-wizard-target': 'personCard',
              person_id: person.id,
              person_name: person.name,
              person_type: person.person_type
            }
          ) do
            span(class: 'text-sm font-black text-foreground') { person.name }
            span(class: 'text-xs font-semibold text-on-surface-variant') do
              t(
                "forms.medications.wizard.dose.person_types.#{person.person_type}",
                default: person.person_type.to_s.humanize
              )
            end
          end
        end

        def render_dose_fields
          dosage = primary_dosage_record_for_wizard

          section(class: 'grid grid-cols-1 sm:grid-cols-2 gap-4') do
            div(class: 'space-y-2') do
              render RubyUI::FormFieldLabel.new(for: 'wizard_dose_amount') do
                t('forms.medications.wizard.dose.amount')
              end
              m3_input(
                type: :number,
                id: 'wizard_dose_amount',
                value: dosage&.amount&.to_s,
                step: 'any',
                min: '0',
                required: true,
                data: {
                  action: 'input->medication-schedule-wizard#update',
                  'medication-schedule-wizard-target': 'amountInput'
                }
              )
            end

            div(class: 'space-y-2') do
              render RubyUI::FormFieldLabel.new(for: 'wizard_dose_unit') do
                t('forms.medications.wizard.dose.unit')
              end
              select(
                id: 'wizard_dose_unit',
                class: 'flex h-14 min-h-[56px] w-full rounded-shape-xs border border-outline bg-transparent ' \
                       'px-4 py-4 text-base transition-all focus-visible:outline-none focus-visible:ring-2 ' \
                       'focus-visible:ring-primary',
                required: true,
                data: {
                  action: 'change->medication-schedule-wizard#update',
                  'medication-schedule-wizard-target': 'unitInput'
                }
              ) do
                option(value: '', selected: dosage&.unit.blank?) { t('forms.medications.select_unit') }
                dosage_units.each do |unit|
                  option(value: unit, selected: dosage&.unit == unit) { unit }
                end
              end
            end
          end
        end

        def render_schedule_type_cards
          section(class: 'space-y-3') do
            m3_heading(level: 4, size: '4', class: 'font-bold tracking-tight text-foreground') do
              t('forms.medications.wizard.dose.schedule_type_title')
            end
            div(class: 'grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3') do
              SCHEDULE_TYPES.each do |type|
                render_schedule_type_card(type)
              end
            end
          end
        end

        def render_schedule_type_card(type)
          selected = type == default_schedule_type
          button(
            type: 'button',
            class: selection_card_classes(selected),
            data: {
              action: 'click->medication-schedule-wizard#selectScheduleType',
              'medication-schedule-wizard-target': 'scheduleTypeCard',
              schedule_type: type
            }
          ) do
            span(class: 'text-sm font-black text-foreground') do
              t("forms.medications.wizard.dose.schedule_types.#{type}.title")
            end
            span(class: 'text-xs font-semibold text-on-surface-variant') do
              t("forms.medications.wizard.dose.schedule_types.#{type}.description")
            end
          end
        end

        def render_follow_up_fields
          section(class: 'space-y-4 rounded-3xl border border-outline-variant/50 bg-surface-container-low p-5') do
            render_multiple_daily_fields
            render_daily_fields
            render_weekly_fields
            render_specific_dates_fields
            render_prn_fields
            render_tapering_fields
          end
        end

        def render_multiple_daily_fields
          div(class: schedule_panel_classes('multiple_daily'), data: schedule_panel_data('multiple_daily')) do
            render_number_input(
              'multiple_daily_count',
              t('forms.medications.wizard.dose.doses_per_day'),
              '2',
              'dosesPerDayInput'
            )
            render_number_input(
              'multiple_daily_hours',
              t('forms.medications.wizard.dose.hours_apart'),
              '12',
              'hoursApartInput'
            )
            div(class: 'grid grid-cols-1 sm:grid-cols-2 gap-4') do
              render_time_input(
                'multiple_daily_first_time',
                t('forms.medications.wizard.dose.first_dose'),
                '08:00',
                'firstTimeInput'
              )
              render_time_input(
                'multiple_daily_second_time',
                t('forms.medications.wizard.dose.second_dose'),
                '20:00',
                'secondTimeInput'
              )
            end
          end
        end

        def render_daily_fields
          div(class: schedule_panel_classes('daily'), data: schedule_panel_data('daily')) do
            render_time_input('daily_time', t('forms.medications.wizard.dose.dose_time'), '08:00', 'dailyTimeInput')
          end
        end

        def render_weekly_fields
          div(class: schedule_panel_classes('weekly'), data: schedule_panel_data('weekly')) do
            div(class: 'grid grid-cols-1 sm:grid-cols-2 gap-4') do
              div(class: 'space-y-2') do
                render RubyUI::FormFieldLabel.new(for: 'weekly_day') do
                  t('forms.medications.wizard.dose.weekly_day')
                end
                select(
                  id: 'weekly_day',
                  class: 'flex h-14 min-h-[56px] w-full rounded-shape-xs border border-outline ' \
                         'bg-transparent px-4 py-4',
                  data: {
                    action: 'change->medication-schedule-wizard#update',
                    'medication-schedule-wizard-target': 'weeklyDayInput'
                  }
                ) do
                  Date::DAYNAMES.each do |day|
                    option(value: day.downcase, selected: day == 'Monday') { day }
                  end
                end
              end
              render_time_input('weekly_time', t('forms.medications.wizard.dose.dose_time'), '08:00', 'weeklyTimeInput')
            end
          end
        end

        def render_specific_dates_fields
          div(class: schedule_panel_classes('specific_dates'), data: schedule_panel_data('specific_dates')) do
            div(class: 'space-y-2') do
              render RubyUI::FormFieldLabel.new(for: 'specific_dates_list') do
                t('forms.medications.wizard.dose.specific_dates')
              end
              m3_input(
                type: :text,
                id: 'specific_dates_list',
                placeholder: t('forms.medications.wizard.dose.specific_dates_placeholder'),
                data: {
                  action: 'input->medication-schedule-wizard#update',
                  'medication-schedule-wizard-target': 'specificDatesInput'
                }
              )
            end
          end
        end

        def render_prn_fields
          div(class: schedule_panel_classes('prn'), data: schedule_panel_data('prn')) do
            div(class: 'grid grid-cols-1 sm:grid-cols-2 gap-4') do
              render_number_input(
                'prn_max_daily',
                t('forms.medications.wizard.dose.max_daily_doses'),
                '4',
                'prnMaxDailyInput'
              )
              render_number_input(
                'prn_hours_apart',
                t('forms.medications.wizard.dose.min_hours_between'),
                '4',
                'prnHoursApartInput'
              )
            end
          end
        end

        def render_tapering_fields
          div(class: schedule_panel_classes('tapering'), data: schedule_panel_data('tapering')) do
            div(class: 'space-y-2') do
              render RubyUI::FormFieldLabel.new(for: 'tapering_plan') do
                t('forms.medications.wizard.dose.tapering_plan')
              end
              render RubyUI::Textarea.new(
                id: 'tapering_plan',
                rows: 3,
                placeholder: t('forms.medications.wizard.dose.tapering_placeholder'),
                data: {
                  action: 'input->medication-schedule-wizard#update',
                  'medication-schedule-wizard-target': 'taperingPlanInput'
                }
              )
            end
          end
        end

        def render_date_fields
          section(class: 'grid grid-cols-1 sm:grid-cols-2 gap-4') do
            render_date_input(
              'wizard_schedule_start_date',
              t('forms.medications.wizard.dose.start_date'),
              Time.zone.today.to_s,
              'startDateInput'
            )
            render_date_input(
              'wizard_schedule_end_date',
              t('forms.medications.wizard.dose.end_date'),
              1.month.from_now.to_date.to_s,
              'endDateInput'
            )
          end
        end

        def render_review_panel
          section(class: 'space-y-3 rounded-3xl bg-primary/5 border border-primary/20 p-5') do
            div(class: 'flex items-start justify-between gap-4') do
              div(class: 'space-y-1') do
                m3_heading(level: 4, size: '4', class: 'font-bold tracking-tight text-foreground') do
                  t('forms.medications.wizard.dose.review_title')
                end
                m3_text(
                  size: '2',
                  class: 'text-on-surface-variant',
                  data: { 'medication-schedule-wizard-target': 'reviewText' }
                ) do
                  t('forms.medications.wizard.dose.review_placeholder')
                end
              end
              m3_button(
                type: :button,
                variant: :outlined,
                class: 'shrink-0',
                data: { action: 'click->medication-schedule-wizard#review' }
              ) do
                t('forms.medications.wizard.dose.review_button')
              end
            end
            input(
              type: 'text',
              id: 'medication_schedule_review_complete',
              name: 'medication_schedule_review_complete',
              required: true,
              tabindex: '-1',
              aria_hidden: 'true',
              class: 'sr-only',
              data: { 'medication-schedule-wizard-target': 'reviewCompleteInput' },
              title: t('forms.medications.wizard.dose.review_required')
            )
          end
        end

        def render_number_input(id, label, value, target)
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(for: id) { label }
            m3_input(
              type: :number,
              id: id,
              value: value,
              min: '0',
              data: {
                action: 'input->medication-schedule-wizard#update',
                'medication-schedule-wizard-target': target
              }
            )
          end
        end

        def render_time_input(id, label, value, target)
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(for: id) { label }
            m3_input(
              type: :time,
              id: id,
              value: value,
              data: {
                action: 'input->medication-schedule-wizard#update',
                'medication-schedule-wizard-target': target
              }
            )
          end
        end

        def render_date_input(id, label, value, target)
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(for: id) { label }
            m3_input(
              type: :date,
              id: id,
              value: value,
              required: true,
              data: {
                action: 'input->medication-schedule-wizard#update',
                'medication-schedule-wizard-target': target
              }
            )
          end
        end

        def selection_card_classes(selected)
          base = 'flex flex-col items-start gap-1 rounded-3xl border p-4 text-left transition-all ' \
                 'hover:border-primary hover:bg-primary/5'
          state = selected ? 'border-primary bg-primary/10 shadow-elevation-1' : 'border-outline-variant/60 bg-surface'
          "#{base} #{state}"
        end

        def schedule_panel_classes(type)
          type == default_schedule_type ? 'space-y-4' : 'space-y-4 hidden'
        end

        def schedule_panel_data(type)
          {
            'medication-schedule-wizard-target': 'schedulePanel',
            schedule_type: type
          }
        end

        def default_schedule_type
          'multiple_daily'
        end

        def selectable_people
          @selectable_people ||= Array(people)
        end

        def selected_person
          selectable_people.first
        end

        def selected_person_id
          selected_person&.id&.to_s
        end

        def primary_dosage_value(attribute)
          primary_dosage_record_for_wizard&.public_send(attribute)
        end

        def default_for_adults_value
          selected_person&.adult? ? '1' : '0'
        end

        def default_for_children_value
          selected_person&.adult? ? '0' : '1'
        end
      end
    end
  end
end
