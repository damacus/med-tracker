# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 write resources' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules

  let(:user) { users(:admin) }
  let(:login_data) { api_login(user) }
  let(:household_id) { login_data.dig('household', 'id') }
  let(:headers) { api_auth_headers(login_data.fetch('access_token')) }

  it 'rejects write requests without bearer authentication' do
    post api_v1_household_people_path(household_id),
         params: { person: { name: 'No Auth', date_of_birth: '1990-01-01' } },
         as: :json

    expect(response).to have_http_status(:unauthorized)
    expect(response.parsed_body.dig('error', 'code')).to eq('unauthorized')
  end

  it 'rejects invalid updated_since filters on collections' do
    get api_v1_household_people_path(household_id),
        params: { updated_since: 'not-a-date' },
        headers: headers,
        as: :json

    expect(response).to have_http_status(:unprocessable_content)
    expect(response.parsed_body.dig('error', 'message')).to eq('updated_since must be ISO8601')
  end

  it 'shows writable resources by id' do
    get api_v1_household_medication_path(household_id, medications(:paracetamol).id),
        headers: headers,
        as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'id')).to eq(medications(:paracetamol).id)

    get api_v1_household_schedule_path(household_id, schedules(:john_paracetamol).id),
        headers: headers,
        as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'id')).to eq(schedules(:john_paracetamol).id)
  end

  it 'creates medication takes idempotently using client_uuid' do
    schedule = schedules(:john_paracetamol)
    client_uuid = SecureRandom.uuid
    payload = {
      medication_take: {
        client_uuid: client_uuid,
        source_type: 'schedule',
        source_id: schedule.id,
        taken_at: '2026-02-25T08:30:00Z'
      }
    }

    expect do
      post api_v1_household_medication_takes_path(household_id), params: payload, headers: headers, as: :json
    end.to change(MedicationTake, :count).by(1)

    expect(response).to have_http_status(:created)
    first_id = response.parsed_body.dig('data', 'id')

    expect do
      post api_v1_household_medication_takes_path(household_id), params: payload, headers: headers, as: :json
    end.not_to change(MedicationTake, :count)

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'id')).to eq(first_id)
  end

  it 'creates medication takes for person medication sources without client_uuid' do
    person_medication = create(
      :person_medication,
      :as_needed,
      person: people(:john),
      medication: medications(:ibuprofen),
      dose_amount: 200,
      dose_unit: 'mg'
    )

    expect do
      post api_v1_household_medication_takes_path(household_id),
           params: {
             medication_take: {
               source_type: 'person_medication',
               source_id: person_medication.id,
               taken_at: '2026-02-25T08:30:00Z'
             }
           },
           headers: headers,
           as: :json
    end.to change(MedicationTake, :count).by(1)

    expect(response).to have_http_status(:created)
    expect(response.parsed_body.dig('data', 'person_medication_id')).to eq(person_medication.id)
  end

  it 'rejects medication takes with invalid timestamps' do
    post api_v1_household_medication_takes_path(household_id),
         params: {
           medication_take: {
             source_type: 'schedule',
             source_id: schedules(:john_paracetamol).id,
             taken_at: 'not-a-time'
           }
         },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:unprocessable_content)
    expect(response.parsed_body.dig('error', 'message')).to eq('taken_at is invalid')
  end

  it 'returns not found for unknown medication take source types' do
    post api_v1_household_medication_takes_path(household_id),
         params: { medication_take: { source_type: 'unknown', source_id: schedules(:john_paracetamol).id } },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:not_found)
  end

  it 'creates and updates people' do
    post api_v1_household_people_path(household_id),
         params: { person: { name: 'API Child', date_of_birth: '2020-01-01', person_type: 'minor' } },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:created)
    person_id = response.parsed_body.dig('data', 'id')
    expect(response.parsed_body.dig('data', 'has_capacity')).to be(false)

    patch api_v1_household_person_path(household_id, person_id),
          params: { person: { name: 'API Child Updated' } },
          headers: headers,
          as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'name')).to eq('API Child Updated')
  end

  it 'creates adults without carer relationships' do
    expect do
      post api_v1_household_people_path(household_id),
           params: { person: { name: 'API Adult', date_of_birth: '1990-01-01', person_type: 'adult' } },
           headers: headers,
           as: :json
    end.not_to change(CarerRelationship, :count)

    expect(response).to have_http_status(:created)
  end

  it 'returns validation errors when updating people with invalid attributes' do
    patch api_v1_household_person_path(household_id, people(:john).id),
          params: { person: { name: '' } },
          headers: headers,
          as: :json

    expect(response).to have_http_status(:unprocessable_content)
    expect(response.parsed_body.dig('error', 'errors')).to include('name')
  end

  it 'creates and updates medications' do
    post api_v1_household_medications_path(household_id),
         params: {
           medication: {
             name: 'API Saline',
             location_id: locations(:home).id,
             dose_amount: 5,
             dose_unit: 'ml',
             current_supply: 100,
             reorder_threshold: 10
           }
         },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:created)
    medication_id = response.parsed_body.dig('data', 'id')

    patch api_v1_household_medication_path(household_id, medication_id),
          params: { medication: { current_supply: 90 } },
          headers: headers,
          as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'current_supply')).to eq('90.0')
  end

  it 'returns validation errors when updating medications with invalid attributes' do
    patch api_v1_household_medication_path(household_id, medications(:paracetamol).id),
          params: { medication: { name: '' } },
          headers: headers,
          as: :json

    expect(response).to have_http_status(:unprocessable_content)
    expect(response.parsed_body.dig('error', 'errors')).to include('name')
  end

  it 'creates and updates schedules' do
    person = people(:john)
    medication = medications(:paracetamol)

    post api_v1_household_schedules_path(household_id),
         params: {
           schedule: {
             person_id: person.id,
             medication_id: medication.id,
             dose_amount: 500,
             dose_unit: 'mg',
             frequency: 'Daily',
             start_date: '2026-02-25',
             end_date: '2026-12-31',
             max_daily_doses: 1,
             dose_cycle: 'daily'
           }
         },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:created)
    schedule_id = response.parsed_body.dig('data', 'id')

    patch api_v1_household_schedule_path(household_id, schedule_id),
          params: { schedule: { frequency: 'Every morning' } },
          headers: headers,
          as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'frequency')).to eq('Every morning')

    patch api_v1_household_schedule_path(household_id, schedule_id),
          params: { schedule: { medication_id: medications(:ibuprofen).id } },
          headers: headers,
          as: :json

    expect(response).to have_http_status(:unprocessable_content)
    expect(response.parsed_body.dig('error', 'errors')).to include('source_dosage_option')
  end

  it 'returns not found when creating schedules for inaccessible medications' do
    other_household = Household.create!(name: 'API Schedule Other Household', slug: 'api-schedule-other-household')
    other_medication = create(:medication, household: other_household)

    post api_v1_household_schedules_path(household_id),
         params: {
           schedule: {
             person_id: people(:john).id,
             medication_id: other_medication.id,
             dose_amount: 500,
             dose_unit: 'mg',
             frequency: 'Daily',
             start_date: '2026-02-25',
             end_date: '2026-12-31'
           }
         },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:not_found)
  end

  it 'creates and updates person medications' do
    person = people(:john)
    medication = medications(:ibuprofen)

    post api_v1_household_person_medications_path(household_id),
         params: {
           person_medication: {
             person_id: person.id,
             medication_id: medication.id,
             dose_amount: 200,
             dose_unit: 'mg',
             administration_kind: 'as_needed'
           }
         },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:created)
    person_medication_id = response.parsed_body.dig('data', 'id')

    patch api_v1_household_person_medication_path(household_id, person_medication_id),
          params: { person_medication: { notes: 'Use with food' } },
          headers: headers,
          as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'notes')).to eq('Use with food')

    patch api_v1_household_person_medication_path(household_id, person_medication_id),
          params: { person_medication: { medication_id: medications(:paracetamol).id } },
          headers: headers,
          as: :json

    expect(response).to have_http_status(:ok)
  end

  it 'returns not found when creating person medications for inaccessible medications' do
    other_household = Household.create!(name: 'API Person Medication Other Household',
                                        slug: 'api-person-medication-other-household')
    other_medication = create(:medication, household: other_household)

    post api_v1_household_person_medications_path(household_id),
         params: {
           person_medication: {
             person_id: people(:john).id,
             medication_id: other_medication.id,
             dose_amount: 200,
             dose_unit: 'mg',
             administration_kind: 'as_needed'
           }
         },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:not_found)
  end

  it 'updates the current notification preference' do
    patch api_v1_household_notification_preference_path(household_id),
          params: { notification_preference: { enabled: false, low_stock_enabled: false } },
          headers: headers,
          as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'enabled')).to be(false)
    expect(response.parsed_body.dig('data', 'low_stock_enabled')).to be(false)
  end

  it 'creates a notification preference when the current person does not have one' do
    user.person.notification_preference&.destroy!

    patch api_v1_household_notification_preference_path(household_id),
          params: { notification_preference: { enabled: true, dose_due_enabled: true } },
          headers: headers,
          as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'dose_due_enabled')).to be(true)
  end

  it 'returns validation errors for invalid write payloads' do
    post api_v1_household_people_path(household_id),
         params: { person: { name: '', date_of_birth: '' } },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:unprocessable_content)
    expect(response.parsed_body.dig('error', 'errors')).to include('name', 'date_of_birth')
  end

  it 'does not update records outside the household scope' do
    other_household = Household.create!(name: 'API Other Household', slug: 'api-other-household')
    other_person = create(:person, household: other_household, name: 'Other Person')

    patch api_v1_household_person_path(household_id, other_person.id),
          params: { person: { name: 'Bad Update' } },
          headers: headers,
          as: :json

    expect(response).to have_http_status(:not_found)
  end
end
