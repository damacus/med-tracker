# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medications::SchedulesSection, type: :component do
  let(:medication) { create(:medication, name: 'Vitamin D') }
  let(:person) { create(:person, name: 'Jane Patient') }
  let(:schedule_set) do
    [
      create(:schedule, medication: medication, person: person, frequency: 'Once daily'),
      create(
        :schedule,
        medication: medication,
        person: person,
        frequency: 'Starts soon',
        start_date: 2.days.from_now.to_date,
        end_date: 1.month.from_now.to_date
      ),
      create(:schedule, :expired, medication: medication, person: person, frequency: 'Ended course'),
      create(
        :schedule,
        medication: medication,
        person: person,
        frequency: 'Stopped course',
        stopped_on: Time.zone.today
      )
    ]
  end

  it 'renders every schedule for the medication with status details' do
    rendered = render_inline(described_class.new(medication: medication, schedules: schedule_set))

    expect(rendered.css("[data-testid='medication-schedules-section']")).to be_present
    expect(rendered.text).to include(
      'Jane Patient',
      'Once daily',
      'Starts soon',
      'Ended course',
      'Stopped course',
      'Active',
      'Future',
      'Ended',
      'Stopped'
    )
  end

  it 'renders editor controls when medication can be updated' do
    rendered = render_with_policy(update: true)

    expect(rendered.css("[data-testid='add-medication-schedule']")).to be_present
    expect(rendered.css("[data-testid='edit-medication-schedule-#{active_schedule.id}']")).to be_present
    expect(rendered.css("[data-testid='stop-medication-schedule-#{active_schedule.id}']")).to be_present
  end

  it 'hides editor controls when medication cannot be updated' do
    rendered = render_with_policy(update: false)

    expect(rendered.css("[data-testid='add-medication-schedule']")).not_to be_present
    expect(rendered.css("[data-testid='edit-medication-schedule-#{active_schedule.id}']")).not_to be_present
    expect(rendered.css("[data-testid='stop-medication-schedule-#{active_schedule.id}']")).not_to be_present
  end

  def render_with_policy(update:)
    policy_stub = Struct.new(:update?).new(update)
    vc = view_context
    vc.singleton_class.define_method(:policy) { |_record| policy_stub }
    html = vc.render(described_class.new(medication: medication, schedules: schedule_set))
    Nokogiri::HTML::DocumentFragment.parse(html)
  end

  def active_schedule = schedule_set.first
end
