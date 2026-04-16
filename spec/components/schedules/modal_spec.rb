# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Schedules::Modal, type: :component do
  subject(:html) do
    render_inline(
      described_class.new(
        schedule: schedule,
        person: person,
        medications: [medication]
      )
    ).to_html
  end

  let(:person) { create(:person, name: 'Damacus User') }
  let(:medication) { create(:medication, name: 'Calpol') }
  let(:schedule) { build(:schedule, person: person, medication: medication) }

  it 'renders a token-driven modal shell for the schedule workflow' do
    expect(html).to include('bg-popover')
    expect(html).to include('bg-foreground/10')
    expect(html).to include('shadow-elevation-5')
    expect(html).not_to include('bg-white')
    expect(html).not_to include('from-[#fffaf1]')
  end
end
