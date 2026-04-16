# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::PersonMedications::FormView, type: :component do
  def rendered_dose_options(component)
    rendered = render_inline(component)
    form = rendered.at_css('form[data-controller="person-medication-form"]')

    expect(form).to be_present
    JSON.parse(form['data-person-medication-form-dose-options-value'])
  end

  describe 'i18n translations' do
    it 'renders form with default locale translations' do
      person_medication = PersonMedication.new
      person = instance_double(Person, name: 'John Doe', person_type: 'adult')
      medications = []

      component = described_class.new(
        person_medication: person_medication,
        person: person,
        medications: medications
      )

      rendered = render_inline(component)

      expect(rendered.to_html).to include('Add Medication')
      expect(rendered.to_html).to include('Add Medication for John Doe')
      expect(rendered.to_html).to include('Cancel')
    end
  end

  it 'renders medication dose options for the Stimulus controller' do
    person_medication = PersonMedication.new
    person = instance_double(Person, name: 'John Doe', person_type: 'adult')
    medication = create(:medication, name: 'Calpol')
    allow(medication).to receive(:dose_options_payload).and_return([{ 'amount' => '1' }])

    payload = rendered_dose_options(
      described_class.new(
        person_medication: person_medication,
        person: person,
        medications: [medication]
      )
    )

    expect(payload).to eq(
      medication.id.to_s => [{ 'amount' => '1' }]
    )
  end
end
