# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::V1::DashboardSerializer do
  fixtures :accounts, :people, :locations, :medications, :dosages, :schedules, :person_medications, :medication_takes

  it 'does not execute SQL while serializing a preloaded dashboard query' do
    date = Date.current
    server_time = Time.current
    query = FamilyDashboard::ScheduleQuery.new([people(:jane)], date: date, now: server_time)
    query.call
    context = described_class::Context.new(
      visible_people: [people(:jane)],
      selected_person: people(:jane),
      schedule_query: query,
      date: date,
      server_time: server_time,
      time_zone: Time.zone.name
    )
    serializer = described_class.new(context)
    queries = captured_sql_queries { serializer.as_json }

    expect(queries).to be_empty
  end

  def captured_sql_queries(&)
    queries = []
    subscriber = lambda do |_name, _start, _finish, _id, payload|
      queries << payload[:sql] unless payload[:cached] || payload[:name] == 'SCHEMA'
    end
    ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record', &)
    queries
  end
end
