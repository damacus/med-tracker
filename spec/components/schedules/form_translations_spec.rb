# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Schedules::Form, type: :component do
  let(:person) { create(:person) }
  let(:medication) { create(:medication, name: 'Ibuprofen') }
  let(:dosage) { create(:dosage, medication:, amount: 200, unit: 'mg', frequency: 'Twice daily') }
  let(:schedule) { Schedule.new(person:, medication:) }

  before do
    dosage
  end

  it 'renders schedule form translations for the Stimulus controller' do
    rendered = render_inline(described_class.new(schedule:, person:, medications: [medication]))
    form = rendered.at_css('form[data-controller="schedule-form"]')

    expect(form).to be_present

    payload = JSON.parse(form['data-schedule-form-translations-value'])

    expect(payload).to include(
      'selectDosage' => I18n.t('schedules.form.select_dosage'),
      'selectMedicationFirst' => I18n.t('schedules.form.select_medication_first'),
      'frequencyOncePerCycle' => I18n.t('schedules.form.frequency_once_per_cycle')
    )
  end

  it 'memoizes duplicate dose selection key computation' do
    duplicate_a = double(selection_key: '200|mg')
    duplicate_b = double(selection_key: '200|mg')
    component = described_class.new(schedule:, person:, medications: [medication])

    allow(medication).to receive(:dosages).and_return([duplicate_a, duplicate_b])

    2.times { component.send(:duplicate_dose_selection_keys) }

    expect(medication).to have_received(:dosages).once
  end
end
