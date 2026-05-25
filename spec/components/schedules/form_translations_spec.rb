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

  it 'renders the derived frequency preview in a Turbo frame' do
    schedule.assign_attributes(max_daily_doses: 3, min_hours_between_doses: 12, dose_cycle: 'weekly')

    rendered = render_inline(described_class.new(schedule:, person:, medications: [medication]))
    preview = rendered.at_css('turbo-frame#schedule_frequency_preview')

    expect(preview).to be_present
    expect(preview['data-schedule-form-target']).to eq('frequencyPreview')
    expect(preview.text).to include('This means:')
    expect(preview.text).to include('Up to 3 times per week, with at least 12 hours between doses')
  end
end
