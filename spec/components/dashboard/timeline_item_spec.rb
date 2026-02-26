# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::TimelineItem, type: :component do
  fixtures :accounts, :people, :locations, :medications, :dosages, :schedules, :person_medications, :medication_takes

  let(:person) { people(:jane) }
  let(:source) { schedules(:jane_ibuprofen) }
  let(:dose) do
    {
      person: person,
      source: source,
      scheduled_at: Time.current,
      taken_at: nil,
      status: :upcoming
    }
  end

  it 'renders the medication name and person name' do
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

  describe 'cooldown badge' do
    let(:source_with_cooldown) do
      schedules(:jane_ibuprofen).tap do |p|
        allow(p).to receive_messages(countdown_display: '3h 30m')
      end
    end

    it 'renders the cooldown badge with countdown text' do
      cooldown_dose = {
        person: person,
        source: source_with_cooldown,
        scheduled_at: 3.hours.from_now,
        taken_at: nil,
        status: :cooldown
      }

      rendered = render_inline(described_class.new(dose: cooldown_dose))

      expect(rendered.to_html).to include('Wait')
      expect(rendered.to_html).to include('3h 30m')
    end

    it 'does not render an action button for cooldown doses' do
      cooldown_dose = {
        person: person,
        source: source_with_cooldown,
        scheduled_at: 3.hours.from_now,
        taken_at: nil,
        status: :cooldown
      }

      rendered = render_inline(described_class.new(dose: cooldown_dose))

      expect(rendered.css('button[type="submit"]')).to be_empty
    end
  end

  describe 'out_of_stock badge' do
    it 'renders the out_of_stock badge' do
      oos_dose = {
        person: person,
        source: source,
        scheduled_at: Time.current,
        taken_at: nil,
        status: :out_of_stock
      }

      rendered = render_inline(described_class.new(dose: oos_dose))

      expect(rendered.to_html).to include('Out of Stock')
    end

    it 'does not render an action button for out_of_stock doses' do
      oos_dose = {
        person: person,
        source: source,
        scheduled_at: Time.current,
        taken_at: nil,
        status: :out_of_stock
      }

      rendered = render_inline(described_class.new(dose: oos_dose))

      expect(rendered.css('button[type="submit"]')).to be_empty
    end
  end

  describe 'upcoming badge with action button' do
    it 'renders an action button for upcoming doses' do
      rendered = render_inline(described_class.new(dose: dose))

      expect(rendered.css('button[type="submit"]')).not_to be_empty
    end
  end
end
