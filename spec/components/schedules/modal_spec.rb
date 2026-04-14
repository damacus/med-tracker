# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Schedules::Modal, type: :component do
  it 'renders a brighter modal shell for the schedule workflow' do
    person = create(:person, name: 'Damacus User')
    medication = create(:medication, name: 'Calpol')
    schedule = build(:schedule, person: person, medication: medication)

    rendered = render_inline(
      described_class.new(
        schedule: schedule,
        person: person,
        medications: [medication]
      )
    )

    html = rendered.to_html

    expect(html).to include('bg-white')
    expect(html).to include('border-border/50')
    expect(html).to include('shadow-[0_32px_90px_rgba(15,23,42,0.18)]')
    expect(html).to include('from-[#fffaf1]')
  end
end
