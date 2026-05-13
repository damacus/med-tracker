# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Schedules::Card, type: :component do
  let(:person) { create(:person) }
  let(:medication) { create(:medication, name: 'Ibuprofen') }

  let(:schedule) do
    Schedule.create!(
      person: person,
      medication: medication,
      dose_amount: 400.0,
      dose_unit: 'mg',
      frequency: 'Twice daily',
      start_date: 1.month.ago,
      end_date: 1.month.from_now
    )
  end

  it 'wraps full medication names inside the card header' do
    medication.update!(name: 'Calpol Six Plus 250mg/5ml oral suspension (McNeil Products Ltd)')
    rendered = render_schedule_card

    heading = rendered.at_css('h3')
    expect(heading.text).to include(medication.name)
    expect(heading['class']).to include('break-words')
  end

  it 'renders a Log a past dose button' do
    rendered = render_schedule_card

    button = rendered.at_css("button[data-testid='log-past-dose-schedule-#{schedule.id}']")

    expect(button).not_to be_nil
    expect(button.text).to include('Log a past dose')
  end

  def render_schedule_card
    vc = view_context
    vc.singleton_class.define_method(:current_user) { nil }
    html = vc.render(described_class.new(schedule: schedule, person: person))
    Nokogiri::HTML::DocumentFragment.parse(html)
  end
end
