# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::PersonMedications::Modal, type: :component do
  def rendered_dose_options(component)
    rendered = render_inline(component)
    form = rendered.at_css('form[data-controller="person-medication-form"]')

    expect(form).to be_present
    JSON.parse(form['data-person-medication-form-dose-options-value'])
  end

  it 'renders a token-driven modal shell for the medication workflow' do
    person = create(:person, name: 'Damacus User')
    medication = create(:medication, name: 'Calpol')
    person_medication = build(:person_medication, person: person, medication: medication)

    rendered = render_inline(
      described_class.new(
        person_medication: person_medication,
        person: person,
        medications: [medication]
      )
    )

    html = rendered.to_html

    expect(html).to include('bg-popover')
    expect(html).to include('bg-foreground/10')
    expect(html).to include('shadow-elevation-5')
    expect(html).not_to include('bg-white')
  end

  it 'renders medication dose options for the Stimulus controller' do
    person = create(:person, name: 'Damacus User')
    medication = create(:medication, name: 'Calpol')
    allow(medication).to receive(:dose_options_payload).and_return([{ 'amount' => '1' }])
    person_medication = build(:person_medication, person: person)

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

  it 'renders the back action as a shared text link' do
    back_link = rendered_back_link
    classes = back_link['class'].split

    expect(back_link.text).to include('Back')
    expect(back_link['data-turbo-frame']).to eq('modal')
    expect(classes).to include('state-layer')
    expect(classes).not_to include('rounded-xl')
  end

  def rendered_back_link
    person = create(:person, name: 'Damacus User')
    medication = create(:medication, name: 'Calpol')
    person_medication = build(:person_medication, person: person, medication: medication)
    back_path = route_helpers.add_medication_person_path(person, household_slug: 'test-household')
    rendered = render_inline(
      described_class.new(person_medication: person_medication, person: person, medications: [medication],
                          back_path: back_path)
    )

    rendered.at_css("a[href='#{back_path}']")
  end

  def route_helpers = Rails.application.routes.url_helpers
end
