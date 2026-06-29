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

  it 'renders pause action for manageable active schedules' do
    rendered = render_schedule_card(manage: true)

    expect(rendered.at_css("button[data-testid='pause-schedule-#{schedule.id}']")).to be_present
  end

  it 'renders paused state without dose actions' do
    schedule.update!(active: false)

    rendered = render_schedule_card(manage: true)

    expect(rendered.text).to include('Paused')
    expect(rendered.at_css("button[data-testid='log-past-dose-schedule-#{schedule.id}']")).to be_nil
    expect(rendered.at_css("button[data-testid='resume-schedule-#{schedule.id}']")).to be_present
  end

  def render_schedule_card(manage: false)
    vc = view_context
    vc.singleton_class.define_method(:current_user) { nil }
    policy_stub = Struct.new(:update?).new(manage)
    allow(SchedulePolicy).to receive(:new).and_return(policy_stub)
    html = vc.render(described_class.new(schedule: schedule, person: person))
    Nokogiri::HTML::DocumentFragment.parse(html)
  end
end
