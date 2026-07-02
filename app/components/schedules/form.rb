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
          data: form_payload
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

      def render_form_fields(_f)
        render Components::Schedules::Fields.new(schedule: schedule, person: person, medications: medications)
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
        input(
          type: :hidden,
          name: 'schedule[source_dosage_option_id]',
          value: schedule.source_dosage_option_id,
          data: { schedule_form_target: 'sourceDosageOptionIdInput' }
        )
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

      def form_payload
        @form_payload ||= ::Schedules::FormPayloadPresenter.new(
          person: person,
          medications: medications,
          frame_id: frame_id,
          urls: {
            next: new_person_schedule_path(person),
            frequency_preview: schedules_frequency_preview_path
          },
          translations: form_translations
        ).data
      end

      def form_translations
        {
          selectDosage: t('schedules.form.select_dosage'),
          selectMedicationFirst: t('schedules.form.select_medication_first'),
          frequencyOncePerCycle: t('schedules.form.frequency_once_per_cycle'),
          frequencyUpToPerCycle: t('schedules.form.frequency_up_to_per_cycle'),
          frequencyOnce: t('schedules.form.frequency_once'),
          frequencyUpTo: t('schedules.form.frequency_up_to'),
          frequencyAtLeastHours: t('schedules.form.frequency_at_least_hours')
        }
      end
    end
  end
end
