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

  describe Components::Schedules::Fields do
    let(:person) { create(:person) }
    let(:medication) { create(:medication, household: person.household) }
    let(:schedule) { create(:schedule, person:, medication:) }

    it 'keeps form-field feedback actions alongside schedule actions' do
      rendered = render_inline(described_class.new(schedule:, person:, medications: [medication]))

      {
        'schedule_medication_id' => 'change->schedule-form#updateDosages',
        'schedule_dose_option_key' => 'change->schedule-form#onDosageChange',
        'schedule_start_date' => 'input->schedule-form#validate',
        'schedule_end_date' => 'input->schedule-form#validate'
      }.each do |input_id, schedule_action|
        input = rendered.at_css("##{input_id}")
        field = input.ancestors.find { |ancestor| ancestor['data-controller'] == 'ruby-ui--form-field' }

        aggregate_failures do
          expect(field.at_css("[data-ruby-ui--form-field-target='error']")).to be_present
          expect(input['data-action']).to include(schedule_action, 'invalid->ruby-ui--form-field#onInvalid')
        end
      end
    end

    it 'renders server-side end date error attributes' do
      schedule.errors.add(:end_date, "can't be blank")

      rendered = render_inline(described_class.new(schedule:, person:, medications: [medication]))
      input = rendered.at_css('#schedule_end_date')

      aggregate_failures do
        expect(input['aria-invalid']).to eq('true')
        expect(input['aria-describedby']).to eq('schedule_end_date_error')
      end
    end
  end
end
