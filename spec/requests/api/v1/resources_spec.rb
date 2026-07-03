# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 resources' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules,
           :person_medications, :medication_takes, :carer_relationships

  let(:user) { users(:admin) }

  before do
    people(:admin).create_notification_preference!(
      enabled: true,
      morning_time: '08:00',
      afternoon_time: '14:00',
      evening_time: '18:00',
      night_time: '22:00'
    )
  end

  it 'returns the current user profile' do
    login_data = api_login(user)
    household_id = login_data.dig('household', 'id')

    get api_v1_household_me_path(household_id), headers: api_auth_headers(login_data.fetch('access_token')), as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'email_address')).to eq(user.email_address)
    expect(response.parsed_body.dig('data', 'account', 'status')).to eq('verified')
  end

  it 'rejects an existing API session bearer token while the account is locked' do
    login_data = api_login(user)
    household_id = login_data.dig('household', 'id')
    AccountLockout.create!(
      account_id: user.person.account.id,
      key: SecureRandom.hex(16),
      deadline: 30.minutes.from_now
    )

    get api_v1_household_me_path(household_id), headers: api_auth_headers(login_data.fetch('access_token')), as: :json

    expect(response).to have_http_status(:unauthorized)
  end

  it 'returns the core read-only collections' do
    login_data = api_login(user)
    household_id = login_data.dig('household', 'id')
    headers = api_auth_headers(login_data.fetch('access_token'))

    {
      api_v1_household_locations_path(household_id) => locations(:home).id,
      api_v1_household_medications_path(household_id) => medications(:paracetamol).id,
      api_v1_household_schedules_path(household_id) => schedules(:john_paracetamol).id,
      api_v1_household_person_medications_path(household_id) => person_medications(:john_vitamin_d).id,
      api_v1_household_medication_takes_path(household_id) => medication_takes(:john_morning_paracetamol).id
    }.each do |path, expected_id|
      get path, headers: headers, as: :json

      expect(response.status).to eq(200), response.parsed_body.merge('path' => path).inspect
      expect(response.parsed_body.fetch('data').map { |row| row.fetch('id') }).to include(expected_id)
      expect(response.parsed_body.fetch('meta')).to include('page' => 1)
    end
  end

  it 'returns the signed-in users notification preference' do
    login_data = api_login(user)
    household_id = login_data.dig('household', 'id')

    get api_v1_household_notification_preference_path(household_id),
        headers: api_auth_headers(login_data.fetch('access_token')),
        as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'person_id')).to eq(people(:admin).id)
    expect(response.parsed_body.dig('data', 'morning_time')).to eq('08:00:00')
  end

  it 'serializes schedule dose snapshot fields instead of dosage identity' do
    login_data = api_login(user)
    household_id = login_data.dig('household', 'id')

    get api_v1_household_schedules_path(household_id),
        headers: api_auth_headers(login_data.fetch('access_token')),
        as: :json

    schedule = response.parsed_body.fetch('data').find { |row| row.fetch('id') == schedules(:john_paracetamol).id }

    expect(schedule).to include(
      'dose_amount' => '1000.0',
      'dose_unit' => 'mg'
    )
    expect(schedule).not_to have_key('dosage_id')
  end

  it 'serializes person medication administration kind' do
    login_data = api_login(user)
    household_id = login_data.dig('household', 'id')

    get api_v1_household_person_medications_path(household_id),
        headers: api_auth_headers(login_data.fetch('access_token')),
        as: :json

    person_medication = response.parsed_body.fetch('data').find do |row|
      row.fetch('id') == person_medications(:john_vitamin_d).id
    end

    expect(person_medication).to include('administration_kind' => 'routine')
  end

  it 'scopes household resource collections to explicitly granted people' do
    scoped_user = users(:jane)
    account = scoped_user.person.account
    household = scoped_user.person.household
    other_household = Household.create!(name: 'Other Resource Household', slug: 'other-resource-household')

    membership = household.household_memberships.find_or_create_by!(
      account: account,
      person: scoped_user.person
    ) do |record|
      record.role = :member
      record.status = :active
    end
    membership.update!(person: scoped_user.person, role: :member, status: :active)
    visible_person = create(:person, household: household, name: 'Alex Resource')
    hidden_person = create(:person, household: household, name: 'Alex Hidden Resource')
    other_person = create(:person, household: other_household, name: 'Alex Other Resource')

    login_data = api_login(scoped_user, household_id: household.id)
    household.person_access_grants.where(household_membership: membership).destroy_all
    household.person_access_grants.create!(
      household_membership: membership,
      person: visible_person,
      access_level: :view,
      relationship_type: :family_member,
      granted_by_membership: membership
    )

    visible_location = create(:location, household: household, name: 'Resource Home')
    hidden_location = create(:location, household: household, name: 'Surgery')
    other_location = create(:location, household: other_household, name: 'Resource Home')
    visible_medication = create(:medication, household: household, location: visible_location, name: 'Paracetamol')
    hidden_medication = create(:medication, household: household, location: hidden_location, name: 'Paracetamol')
    other_medication = create(:medication, household: other_household, location: other_location, name: 'Paracetamol')
    visible_dosage = create(:dosage, household: household, medication: visible_medication)
    hidden_dosage = create(:dosage, household: household, medication: hidden_medication)
    other_dosage = create(:dosage, household: other_household, medication: other_medication)
    visible_schedule = create(
      :schedule,
      household: household,
      person: visible_person,
      medication: visible_medication,
      dosage: visible_dosage
    )
    hidden_schedule = create(
      :schedule,
      household: household,
      person: hidden_person,
      medication: hidden_medication,
      dosage: hidden_dosage
    )
    other_schedule = create(
      :schedule,
      household: other_household,
      person: other_person,
      medication: other_medication,
      dosage: other_dosage
    )
    visible_person_medication = create(
      :person_medication,
      household: household,
      person: visible_person,
      medication: visible_medication,
      dosage: visible_dosage
    )
    hidden_person_medication = create(
      :person_medication,
      household: household,
      person: hidden_person,
      medication: hidden_medication,
      dosage: hidden_dosage
    )
    visible_take = create(:medication_take, :for_schedule, household: household, schedule: visible_schedule)
    hidden_take = create(:medication_take, :for_schedule, household: household, schedule: hidden_schedule)
    other_take = create(:medication_take, :for_schedule, household: other_household, schedule: other_schedule)

    headers = api_auth_headers(login_data.fetch('access_token'))

    {
      "/api/v1/households/#{household.id}/medications" => [visible_medication.id, hidden_medication.id,
                                                           other_medication.id],
      "/api/v1/households/#{household.id}/schedules" => [visible_schedule.id, hidden_schedule.id, other_schedule.id],
      "/api/v1/households/#{household.id}/person_medications" => [
        visible_person_medication.id,
        hidden_person_medication.id
      ],
      "/api/v1/households/#{household.id}/medication_takes" => [visible_take.id, hidden_take.id, other_take.id]
    }.each do |path, ids|
      get path, headers: headers, as: :json

      expect(response.status).to eq(200), response.parsed_body.merge('path' => path).inspect
      returned_ids = response.parsed_body.fetch('data').map { |row| row.fetch('id') }
      expect(returned_ids).to include(ids.first)
      expect(returned_ids).not_to include(*ids.drop(1))
    end
  end
end
