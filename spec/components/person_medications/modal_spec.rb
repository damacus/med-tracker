# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::PersonMedications::Modal, type: :component do
  it 'renders a brighter modal shell for the medication workflow' do
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

    expect(html).to include('bg-white')
    expect(html).to include('border-border/50')
    expect(html).to include('shadow-[0_32px_90px_rgba(15,23,42,0.18)]')
  end
end
