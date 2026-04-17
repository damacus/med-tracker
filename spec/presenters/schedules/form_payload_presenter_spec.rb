# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Schedules::FormPayloadPresenter do
  describe '#data' do
    let(:person) { instance_double(Person, person_type: 'adult') }
    let(:medication) do
      instance_double(Medication, id: 123, dose_options_payload: [{ 'amount' => '1' }])
    end
    let(:translations) do
      {
        selectDosage: 'Select dosage',
        selectMedicationFirst: 'Select medication first',
        frequencyAtLeastHours: 'At least %<hours>s hours apart'
      }
    end

    def build_presenter
      described_class.new(
        person: person,
        medications: [medication],
        frame_id: 'schedule_frame',
        next_url: '/people/1/schedules/new',
        translations: translations
      )
    end

    it 'includes the core stimulus metadata' do
      expect(build_presenter.data).to include(
        controller: 'schedule-form',
        turbo_stream: true,
        person_type: 'adult',
        schedule_form_frame_id_value: 'schedule_frame',
        schedule_form_next_url_value: '/people/1/schedules/new'
      )
    end

    it 'builds the schedule form stimulus payload' do
      data = build_presenter.data

      expect(data[:schedule_form_dose_options_value]).to eq({ '123' => [{ 'amount' => '1' }] }.to_json)
      expect(JSON.parse(data[:schedule_form_translations_value])).to include(
        'selectDosage' => 'Select dosage',
        'selectMedicationFirst' => 'Select medication first',
        'frequencyAtLeastHours' => 'At least %<hours>s hours apart'
      )
    end
  end
end
