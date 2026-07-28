# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::ExperimentalBaseView, type: :component do
  fixtures :accounts, :people, :users, :locations, :medications, :dosages, :schedules,
           :person_medications, :medication_takes

  let(:person) { people(:john) }
  let(:current_user) { users(:admin) }
  let(:row) do
    {
      person: person,
      source: person_medications(:john_vitamin_d),
      scheduled_at: Time.zone.parse('2026-07-19 09:30:00'),
      taken_at: nil,
      status: :upcoming,
      daily_dose_count: 0,
      daily_dose_limit: 1,
      today_takes: []
    }
  end
  let(:presenter) do
    instance_double(
      DashboardPresenter,
      people: [person],
      active_schedules: [schedules(:john_paracetamol)],
      current_user: current_user,
      selected_person_id: nil,
      dashboard_person_options: [],
      routine_tasks_by_person: { person => [row] },
      as_needed_by_person: { person => [] },
      today_takes_by_person: { person => [medication_takes(:john_morning_paracetamol)] },
      next_due_value: '09:30',
      due_now_count: 1,
      tasks_left_count: 1
    )
  end

  it 'renders the time-first design as a chronological dashboard' do
    component = 'Components::Dashboard::TimeFirstView'.constantize.new(presenter: presenter)

    rendered = render_inline(component)

    expect(rendered.at_css('[data-testid="dashboard-variant-time-first"]')).to be_present
    expect(rendered.text).to include('Next up')
    expect(rendered.text).to include('Morning')
    expect(rendered.text).to include('Later today')
    expect(rendered.text).to include('Vitamin D')
  end

  it 'renders the family-lanes design with a working time grouping link', :aggregate_failures do
    component = 'Components::Dashboard::FamilyLanesView'.constantize.new(presenter: presenter, grouping: 'person')

    rendered = render_inline(component)

    expect(rendered.at_css('[data-testid="dashboard-variant-family-lanes"]')).to be_present
    expect(rendered.text).to include("Today's care")
    expect(rendered.text).to include('By person')
    expect(rendered.text).to include('By time')
    expect(rendered.at_css('a[href*="dashboard_grouping=time"]')).to be_present
    expect(rendered.text).to include(person.name)
  end

  it 'renders family lanes chronologically when time grouping is selected' do
    component = 'Components::Dashboard::FamilyLanesView'.constantize.new(presenter: presenter, grouping: 'time')

    rendered = render_inline(component)

    expect(rendered.at_css('[data-testid="dashboard-family-time"]')).to be_present
    expect(rendered.text).to include('Morning')
  end

  it 'renders the calm-focus design around one safe next action' do
    component = 'Components::Dashboard::CalmFocusView'.constantize.new(presenter: presenter)

    rendered = render_inline(component)

    expect(rendered.at_css('[data-testid="dashboard-variant-calm-focus"]')).to be_present
    expect(rendered.text).to include('Needs attention now')
    expect(rendered.text).to include('After this')
    expect(rendered.text).to include('Review & record')
    expect(rendered.text).to include('Completed today')
  end
end
