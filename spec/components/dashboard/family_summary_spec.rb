# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::FamilySummary, type: :component do
  fixtures :all

  let(:jane) { people(:jane) }
  let(:doses) do
    [
      {
        person: jane,
        source: prescriptions(:jane_ibuprofen),
        scheduled_at: Time.current,
        taken_at: nil,
        status: :upcoming
      }
    ]
  end

  it 'renders the dashboard title' do
    component = described_class.new(doses: doses)
    rendered = render_inline(component)

    expect(rendered.to_html).to include('Family Dashboard')
  end

  it 'renders timeline items for doses' do
    component = described_class.new(doses: doses)
    rendered = render_inline(component)

    expect(rendered.to_html).to include('Ibuprofen')
  end

  it 'renders an empty state message when no doses' do
    component = described_class.new(doses: [])
    rendered = render_inline(component)

    expect(rendered.to_html).to include('No medications scheduled for today')
  end
end
