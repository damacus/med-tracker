# frozen_string_literal: true

module Schedules
  class FormPayloadPresenter
    attr_reader :person, :medications, :frame_id, :next_url, :translations

    def initialize(person:, medications:, frame_id:, next_url:, translations:)
      @person = person
      @medications = medications
      @frame_id = frame_id
      @next_url = next_url
      @translations = translations
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
        schedule_form_next_url_value: next_url,
        schedule_form_translations_value: translations.to_json
      }
    end
  end
end
