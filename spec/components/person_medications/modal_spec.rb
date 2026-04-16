# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::PersonMedications::Modal, type: :component do
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
end
