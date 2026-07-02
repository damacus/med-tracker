# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::PersonSchedule, type: :component do
  fixtures :accounts, :people, :users, :locations, :medications, :dosages, :schedules

  subject(:component) { described_class.new(person: person, schedules: active_schedules) }

  let(:person) { people(:john) }
  let(:active_schedules) { person.schedules.where(active: true) }

  before do
    MedicationTake.where(schedule: active_schedules).delete_all
  end

  it 'renders the person\'s name' do
    rendered = render_inline(component)
    expect(rendered.text).to include(person.name)
  end

  it 'renders each schedule' do
    rendered = render_inline(component)

    active_schedules.each do |schedule|
      schedule_element = rendered.css("#schedule_#{schedule.id}")
      expect(schedule_element).to be_present
      expect(rendered.text).to include(schedule.medication.display_name)
    end
  end

  it 'renders take now links for each schedule' do
    rendered = render_inline(component)

    active_schedules.each do |schedule|
      link = rendered.css("[data-test-id='take-medication-#{schedule.id}']")
      expect(link).to be_present
      expect(link.text).to include('Take Now')
    end
  end

  it 'does not repeat the dashboard blocked-state stock lookup when a schedule is not blocked' do
    schedule = schedules(:active_schedule)

    expect(count_stock_source_queries do
      render_inline(described_class.new(person: person, schedules: [schedule]))
    end).to eq(2)
  end

  def count_stock_source_queries(&)
    count = 0
    subscriber = lambda do |_name, _started, _finished, _unique_id, payload|
      sql = payload[:sql]
      count += 1 if sql.include?('FROM "medications"') &&
                    sql.include?('"medications"."name"') &&
                    sql.include?('"medications"."dose_amount"')
    end

    ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record', &)
    count
  end
end
