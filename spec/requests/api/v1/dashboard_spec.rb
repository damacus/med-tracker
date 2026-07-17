# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 dashboard' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules,
           :person_medications, :medication_takes, :carer_relationships

  let(:user) { users(:jane) }

  it 'returns the typed household Today dashboard' do
    login_data = api_login(user)
    household_id = login_data.dig('household', 'id')

    get "/api/v1/households/#{household_id}/dashboard",
        params: { date: '2026-07-17', person_id: 'all' },
        headers: api_auth_headers(login_data.fetch('access_token')),
        as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'format')).to eq('medtracker.dashboard.v1')
  end

  %i[admin doctor jane carer parent].each do |user_name|
    it "returns only policy-visible people for #{user_name}" do
      selected_user = users(user_name)
      login_data = api_login(selected_user)
      household_id = login_data.dig('household', 'id')
      membership = HouseholdMembership.find_by!(household_id:, account: selected_user.person.account)
      expected_ids = Person.where(
        id: PersonAccessGrant.active.where(household_membership: membership).select(:person_id)
      ).order(:id).pluck(:portable_id)

      get "/api/v1/households/#{household_id}/dashboard",
          params: { person_id: 'all' },
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      returned_ids = response.parsed_body.dig('data', 'people').pluck('id')
      expect(response).to have_http_status(:ok)
      expect(returned_ids).to eq(expected_ids)
    end
  end

  it 'returns one selected visible person without hiding the visible-person selector' do
    login_data = api_login(user)
    household_id = login_data.dig('household', 'id')
    selected = user.person

    get "/api/v1/households/#{household_id}/dashboard",
        params: { person_id: selected.portable_id },
        headers: api_auth_headers(login_data.fetch('access_token')),
        as: :json

    data = response.parsed_body.fetch('data')
    task_people = (data.fetch('routine_tasks') + data.fetch('as_needed_tasks')).pluck('person').pluck('id').uniq
    expect(response).to have_http_status(:ok)
    expect(data.dig('selected_person', 'id')).to eq(selected.portable_id)
    expect(task_people).to all(eq(selected.portable_id))
    expect(data.fetch('people').size).to be >= 1
  end

  it 'rejects hidden and unknown person filters with the same response' do
    login_data = api_login(user)
    household_id = login_data.dig('household', 'id')
    membership = HouseholdMembership.find_by!(household_id:, account: user.person.account)
    granted_ids = PersonAccessGrant.active.where(household_membership: membership).select(:person_id)
    hidden_person = Person.where(household_id:).where.not(id: granted_ids).first!
    responses = [hidden_person.portable_id, SecureRandom.uuid].map do |person_id|
      get "/api/v1/households/#{household_id}/dashboard",
          params: { person_id: person_id },
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json
      [response.status, response.parsed_body]
    end

    expect(responses.map(&:first)).to eq([422, 422])
    expect(responses.map { |result| result.last.dig('error', 'message') }.uniq).to eq(['person_id is invalid'])
  end

  it 'rejects non-strict and impossible dates' do
    login_data = api_login(user)
    household_id = login_data.dig('household', 'id')

    %w[17-07-2026 2026-02-30].each do |date|
      get "/api/v1/households/#{household_id}/dashboard",
          params: { date: date },
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.parsed_body.dig('error', 'message')).to eq('date must be YYYY-MM-DD')
    end
  end

  it 'uses the household timezone for the requested date boundary' do
    login_data = api_login(user)
    household = Household.find(login_data.dig('household', 'id'))
    household.update!(timezone: 'Europe/London')
    source = create_dashboard_schedule(
      household: household,
      person: user.person,
      name: 'Boundary medication',
      schedule_type: :prn,
      frequency: 'As needed'
    )
    included_take = create(
      :medication_take,
      schedule: source,
      person_medication: nil,
      taken_at: Time.iso8601('2026-07-16T23:30:00Z'),
      skip_stock_mutation: true
    )
    excluded_take = create(
      :medication_take,
      schedule: source,
      person_medication: nil,
      taken_at: Time.iso8601('2026-07-17T23:30:00Z'),
      skip_stock_mutation: true
    )

    get "/api/v1/households/#{household.id}/dashboard",
        params: { date: '2026-07-17', person_id: user.person.portable_id },
        headers: api_auth_headers(login_data.fetch('access_token')),
        as: :json

    data = response.parsed_body.fetch('data')
    returned_ids = data.fetch('recent_completed_takes').pluck('portable_id')
    expect(data['time_zone']).to eq('Europe/London')
    expect(returned_ids).to include(included_take.portable_id)
    expect(returned_ids).not_to include(excluded_take.portable_id)
  end

  it 'preserves routine, PRN, blocking, paused, and stock-selection states' do
    travel_to Time.zone.parse('2026-07-17 12:00:00') do
      login_data = api_login(users(:admin))
      household = Household.find(login_data.dig('household', 'id'))
      person = users(:admin).person
      due = create_dashboard_schedule(household:, person:, name: 'Due tablet', times: ['08:00'])
      upcoming = create_dashboard_schedule(household:, person:, name: 'Upcoming tablet', times: ['20:00'])
      available = create_dashboard_schedule(
        household:, person:, name: 'Available spray', schedule_type: :prn, frequency: 'As needed'
      )
      cooldown = create_dashboard_schedule(
        household:, person:, name: 'Cooldown spray', schedule_type: :prn, frequency: 'As needed',
        min_hours_between_doses: 4
      )
      max_reached = create_dashboard_schedule(
        household:, person:, name: 'Limited spray', schedule_type: :prn, frequency: 'As needed',
        min_hours_between_doses: 4, max_daily_doses: 1
      )
      paused = create_dashboard_schedule(
        household:, person:, name: 'Paused spray', schedule_type: :prn, frequency: 'As needed'
      ).tap(&:pause!)
      out_of_stock = create_dashboard_schedule(
        household:, person:, name: 'Empty spray', schedule_type: :prn, frequency: 'As needed', current_supply: 0
      )
      selection = create_selection_schedule(household:, person:)
      [cooldown, max_reached].each do |source|
        create(:medication_take, schedule: source, person_medication: nil, taken_at: 1.hour.ago,
                                 skip_stock_mutation: true)
      end

      get "/api/v1/households/#{household.id}/dashboard",
          params: { date: '2026-07-17', person_id: person.portable_id },
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      data = response.parsed_body.fetch('data')
      tasks = (data.fetch('routine_tasks') + data.fetch('as_needed_tasks')).index_by { |task| task['source_id'] }
      expect(tasks.fetch(due.portable_id)).to include('status' => 'due', 'can_record' => true)
      expect(tasks.fetch(upcoming.portable_id)).to include('status' => 'upcoming', 'can_record' => true)
      expect(tasks.fetch(available.portable_id)).to include('status' => 'available', 'can_record' => true)
      expect(tasks.fetch(cooldown.portable_id)).to include('status' => 'cooldown', 'blocking_reason' => 'cooldown')
      expect(tasks.fetch(max_reached.portable_id)).to include('status' => 'max_reached',
                                                              'blocking_reason' => 'max_reached')
      expect(tasks.fetch(paused.portable_id)).to include('status' => 'paused', 'blocking_reason' => 'paused')
      expect(tasks.fetch(out_of_stock.portable_id)).to include('status' => 'out_of_stock',
                                                               'blocking_reason' => 'out_of_stock')
      expect(tasks.fetch(selection.portable_id)).to include('status' => 'selection_required',
                                                            'blocking_reason' => 'selection_required',
                                                            'can_record' => false)
      expect(tasks.fetch(selection.portable_id).fetch('stock_source_choices').size).to eq(2)
      expect(tasks.values).to all(include('person', 'medication', 'dose', 'daily_progress', 'scheduled_at'))
      expect(data.fetch('recent_completed_takes')).to all(include('reversal' => nil))
      expect(response.body.bytesize).to be <= 150.kilobytes
    end
  end

  it 'sets can_record from person policy access' do
    login_data = api_login(users(:doctor))
    household = Household.find(login_data.dig('household', 'id'))
    membership = HouseholdMembership.find_by!(household:, account: users(:doctor).person.account)
    view_grant = PersonAccessGrant.active.find_by!(household_membership: membership, access_level: :view)
    source = create_dashboard_schedule(
      household: household,
      person: view_grant.person,
      name: 'Clinician view-only spray',
      schedule_type: :prn,
      frequency: 'As needed'
    )

    get "/api/v1/households/#{household.id}/dashboard",
        params: { person_id: view_grant.person.portable_id },
        headers: api_auth_headers(login_data.fetch('access_token')),
        as: :json

    tasks = response.parsed_body.dig('data', 'as_needed_tasks').index_by { |task| task['source_id'] }
    expect(tasks.fetch(source.portable_id)).to include('status' => 'available', 'can_record' => false)
  end

  it 'does not expose hidden matching stock sources' do
    login_data = api_login(user)
    household = Household.find(login_data.dig('household', 'id'))
    membership = HouseholdMembership.find_by!(household:, account: user.person.account)
    granted_ids = PersonAccessGrant.active.where(household_membership: membership).select(:person_id)
    hidden_person = Person.where(household: household).where.not(id: granted_ids).first!
    source = create_dashboard_schedule(
      household: household,
      person: user.person,
      name: 'Private matching spray',
      schedule_type: :prn,
      frequency: 'As needed'
    )
    hidden_medication = create(
      :medication,
      household: household,
      location: create(:location, household: household),
      name: source.medication.name,
      dose_amount: source.medication.dose_amount,
      dose_unit: source.medication.dose_unit,
      current_supply: 10
    )
    create(
      :schedule,
      household: household,
      person: hidden_person,
      medication: hidden_medication,
      dosage: nil,
      dose_amount: hidden_medication.dose_amount,
      dose_unit: hidden_medication.dose_unit,
      schedule_type: :prn,
      frequency: 'As needed'
    )

    get "/api/v1/households/#{household.id}/dashboard",
        params: { person_id: user.person.portable_id },
        headers: api_auth_headers(login_data.fetch('access_token')),
        as: :json

    tasks = response.parsed_body.dig('data', 'as_needed_tasks').index_by { |task| task['source_id'] }
    expect(tasks.fetch(source.portable_id)).to include('status' => 'available', 'stock_source_choices' => [])
  end

  it 'keeps the representative dashboard within the SQL query budget' do
    login_data = api_login(user)
    household_id = login_data.dig('household', 'id')
    query_count = count_sql_queries do
      get "/api/v1/households/#{household_id}/dashboard",
          params: { person_id: user.person.portable_id },
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json
    end

    expect(response).to have_http_status(:ok)
    expect(query_count).to be <= 20
  end

  def create_dashboard_schedule(household:, person:, name:, **overrides)
    options = dashboard_schedule_options.merge(overrides)
    location = create(:location, household: household)
    medication = create(
      :medication,
      household: household,
      location: location,
      name: name,
      current_supply: options[:current_supply]
    )

    create(:schedule, **dashboard_schedule_attributes(household:, person:, medication:, options:))
  end

  def dashboard_schedule_options
    {
      times: nil,
      schedule_type: :daily,
      frequency: 'Daily',
      current_supply: 20,
      max_daily_doses: 4,
      min_hours_between_doses: nil
    }
  end

  def dashboard_schedule_attributes(household:, person:, medication:, options:)
    {
      household: household,
      person: person,
      medication: medication,
      dosage: nil,
      dose_amount: medication.dose_amount,
      dose_unit: medication.dose_unit,
      schedule_type: options[:schedule_type],
      frequency: options[:frequency],
      schedule_config: options[:times] ? { 'times' => options[:times] } : {},
      max_daily_doses: options[:max_daily_doses],
      min_hours_between_doses: options[:min_hours_between_doses],
      start_date: Date.new(2026, 1, 1),
      end_date: Date.new(2026, 12, 31)
    }
  end

  def create_selection_schedule(household:, person:)
    first = create_dashboard_schedule(
      household: household,
      person: person,
      name: 'Shared rescue spray',
      schedule_type: :prn,
      frequency: 'As needed'
    )
    create_matching_medication(household:, medication: first.medication)
    first
  end

  def create_matching_medication(household:, medication:)
    create(
      :medication,
      household: household,
      location: create(:location, household: household),
      name: medication.name,
      dose_amount: medication.dose_amount,
      dose_unit: medication.dose_unit,
      current_supply: 10
    )
  end

  def count_sql_queries(&)
    count = 0
    subscriber = lambda do |_name, _start, _finish, _id, payload|
      table_pattern = Regexp.union(domain_tables)
      relevant_table = payload[:sql].match?(/(?:FROM|JOIN) "(?:#{table_pattern})"/)
      count += 1 if relevant_table && !payload[:cached] && payload[:name] != 'SCHEMA'
    end
    ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record', &)
    count
  end

  def domain_tables
    %w[people person_access_grants schedules person_medications medication_takes medications locations]
  end
end
