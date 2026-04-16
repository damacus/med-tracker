# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Schedules::FormPayloadPresenter do
  describe '#data' do
    let(:person) { instance_double(Person, person_type: 'adult') }
    let(:medication) do
      instance_double(Medication, id: 123, dose_options_payload: [{ 'amount' => '1' }])
    end
    let(:view_context) do
      instance_double(
        Components::Schedules::Form,
        new_person_schedule_path: '/people/1/schedules/new'
      )
    end

    before do
      allow(view_context).to receive(:t) { |key| I18n.t(key) }
    end

    it 'includes the core stimulus metadata' do
      data = described_class.new(
        person: person,
        medications: [medication],
        frame_id: 'schedule_frame',
        view_context: view_context
      ).data

      expect(data).to include(
        controller: 'schedule-form',
        turbo_stream: true,
        person_type: 'adult',
        schedule_form_frame_id_value: 'schedule_frame',
        schedule_form_next_url_value: '/people/1/schedules/new'
      )
    end

    it 'builds the schedule form stimulus payload' do
      data = described_class.new(
        person: person,
        medications: [medication],
        frame_id: 'schedule_frame',
        view_context: view_context
      ).data

      expect(data[:schedule_form_dose_options_value]).to eq({ '123' => [{ 'amount' => '1' }] }.to_json)
      expect(JSON.parse(data[:schedule_form_translations_value])).to include(
        'selectDosage' => I18n.t('schedules.form.select_dosage'),
        'selectMedicationFirst' => I18n.t('schedules.form.select_medication_first'),
        'frequencyAtLeastHours' => I18n.t('schedules.form.frequency_at_least_hours')
      )
    end
  end
end
