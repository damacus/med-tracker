# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::V1::DashboardSerializer do
  fixtures :accounts, :people, :locations, :medications, :dosages, :schedules, :person_medications, :medication_takes

  before { FixtureHouseholdSetup.apply! }

  it 'does not execute SQL while serializing preloaded family tasks, stock choices, and takes' do
    serializer, source = serializer_and_source
    payload = nil
    queries = captured_sql_queries { payload = serializer.as_json }

    expect(queries).to be_empty
    task = payload.fetch(:as_needed_tasks).find { |candidate| candidate.fetch(:source_id) == source.portable_id }
    expect(task.fetch(:stock_source_choices).size).to eq(2)
    expect(payload.fetch(:recent_completed_takes)).not_to be_empty
  end

  def serializer_and_source
    date = Date.current
    server_time = Time.current
    household = people(:jane).household
    selected_people = [people(:jane), people(:john)]
    source = create_selection_source(household:, person: people(:jane))
    query = loaded_schedule_query(household, selected_people, date, server_time)
    context = serializer_context(selected_people, query, date, server_time)
    [described_class.new(context), source]
  end

  def loaded_schedule_query(household, selected_people, date, server_time)
    query = FamilyDashboard::ScheduleQuery.new(
      selected_people,
      current_user: authorization_context(household),
      date: date,
      now: server_time,
      include_paused: true
    )
    query.call
    query
  end

  def serializer_context(selected_people, query, date, server_time)
    described_class::Context.new(
      visible_people: selected_people,
      selected_person: people(:jane),
      schedule_query: query,
      date: date,
      server_time: server_time,
      time_zone: Time.zone.name
    )
  end

  def authorization_context(household)
    account = people(:admin).account
    membership = household.household_memberships.find_by!(account: account)
    AuthorizationContext.new(account: account, household: household, membership: membership)
  end

  def create_selection_source(household:, person:)
    medication = create(:medication, household: household, name: 'Serializer selection medication')
    create(:medication, **matching_medication_attributes(household, medication))
    create(:schedule, **selection_schedule_attributes(household, person, medication))
  end

  def matching_medication_attributes(household, medication)
    {
      household: household,
      name: medication.name,
      dose_amount: medication.dose_amount,
      dose_unit: medication.dose_unit
    }
  end

  def selection_schedule_attributes(household, person, medication)
    {
      household: household,
      person: person,
      medication: medication,
      dosage: nil,
      dose_amount: medication.dose_amount,
      dose_unit: medication.dose_unit,
      schedule_type: :prn,
      frequency: 'As needed',
      min_hours_between_doses: nil
    }
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
