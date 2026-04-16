# frozen_string_literal: true

module Schedules
  class FormPayloadPresenter
    attr_reader :person, :medications, :frame_id, :view_context

    def initialize(person:, medications:, frame_id:, view_context:)
      @person = person
      @medications = medications
      @frame_id = frame_id
      @view_context = view_context
    end

    def data
      {
        controller: 'schedule-form',
        turbo_stream: true,
        person_type: person.person_type,
        schedule_form_dose_options_value: ::Medications::DoseOptionsPayloadPresenter.new(
          medications: medications
        ).to_h.to_json,
        schedule_form_frame_id_value: frame_id,
        schedule_form_next_url_value: view_context.new_person_schedule_path(person),
        schedule_form_translations_value: translations.to_json
      }
    end

    private

    def translations
      {
        selectDosage: view_context.t('schedules.form.select_dosage'),
        selectMedicationFirst: view_context.t('schedules.form.select_medication_first'),
        frequencyOncePerCycle: view_context.t('schedules.form.frequency_once_per_cycle'),
        frequencyUpToPerCycle: view_context.t('schedules.form.frequency_up_to_per_cycle'),
        frequencyOnce: view_context.t('schedules.form.frequency_once'),
        frequencyUpTo: view_context.t('schedules.form.frequency_up_to'),
        frequencyAtLeastHours: view_context.t('schedules.form.frequency_at_least_hours')
      }
    end
  end
end
