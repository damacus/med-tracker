# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::TimelineItem, type: :component do
  fixtures :people, :locations, :medicines, :prescriptions, :person_medicines, :medication_takes

  let(:person) { people(:jane) }
  let(:source) { prescriptions(:jane_ibuprofen) }
  let(:dose) do
    {
      person: person,
      source: source,
      scheduled_at: Time.current,
      taken_at: nil,
      status: :upcoming
    }
  end

  it 'renders the medicine name and person name' do
    component = described_class.new(dose: dose)
    rendered = render_inline(component)

    expect(rendered.to_html).to include('Ibuprofen')
    expect(rendered.to_html).to include('Jane Doe')
  end

  it 'renders the correct status badge' do
    component = described_class.new(dose: dose)
    rendered = render_inline(component)

    expect(rendered.to_html).to include('Upcoming')
  end

  it 'renders a success badge when taken' do
    dose[:status] = :taken
    dose[:taken_at] = Time.current

    component = described_class.new(dose: dose)
    rendered = render_inline(component)

    expect(rendered.to_html).to include('Taken')
    expect(rendered.to_html).to include('Taken at')
  end

  it 'shows only the person name for upcoming doses' do
    component = described_class.new(dose: dose)
    rendered = render_inline(component)

    expect(rendered.to_html).to include('Jane Doe')
    expect(rendered.to_html).not_to include('Taken at')
  end
end
