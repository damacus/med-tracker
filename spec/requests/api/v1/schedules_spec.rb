# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 schedules' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules

  let(:user) { users(:admin) }
  let(:login_data) { api_login(user) }
  let(:household_id) { login_data.dig('household', 'id') }
  let(:headers) { api_auth_headers(login_data.fetch('access_token')) }

  describe 'GET /api/v1/households/:household_id/schedules' do
    it 'returns the schedules in the signed-in user scope' do
      get api_v1_household_schedules_path(household_id),
          headers: headers,
          as: :json

      expect(response).to have_http_status(:ok)

      returned_ids = response.parsed_body.fetch('data').map { |schedule| schedule.fetch('id') }
      expect(returned_ids).to include(schedules(:john_paracetamol).id)
    end
  end

  describe 'GET /api/v1/households/:household_id/schedules/:id' do
    it 'returns a specific schedule' do
      schedule = schedules(:john_paracetamol)

      get api_v1_household_schedule_path(household_id, schedule.id),
          headers: headers,
          as: :json

      expect(response).to have_http_status(:ok)
      expect(response.parsed_body.dig('data', 'id')).to eq(schedule.id)
    end

    it 'returns not found for a schedule outside scope' do
      other_household = Household.create!(name: 'API Schedule Other Household', slug: 'api-schedule-other-household')
      other_medication = create(:medication, household: other_household)
      other_person = create(:person, household: other_household)
      other_schedule = create(:schedule, household: other_household, person: other_person, medication: other_medication)

      get api_v1_household_schedule_path(household_id, other_schedule.id),
          headers: headers,
          as: :json

      expect(response).to have_http_status(:not_found)
    end
  end

  describe 'POST /api/v1/households/:household_id/schedules' do
    it 'creates a new schedule' do
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
      expect(response.parsed_body.dig('data', 'dose_amount')).to eq('500.0')
    end

    it 'returns validation errors for invalid payload' do
      person = people(:john)
      medication = medications(:paracetamol)

      post api_v1_household_schedules_path(household_id),
           params: {
             schedule: {
               person_id: person.id,
               medication_id: medication.id,
               start_date: '2026-02-25'
             }
           },
           headers: headers,
           as: :json

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.parsed_body.dig('error', 'errors')).to include('schedule_type')
    end
  end

  describe 'PATCH /api/v1/households/:household_id/schedules/:id' do
    it 'updates an existing schedule' do
      schedule = schedules(:john_paracetamol)

      patch api_v1_household_schedule_path(household_id, schedule.id),
            params: {
              schedule: {
                frequency: 'Every 8 hours'
              }
            },
            headers: headers,
            as: :json

      expect(response).to have_http_status(:ok)
      expect(response.parsed_body.dig('data', 'frequency')).to eq('Every 8 hours')
    end

    it 'returns validation errors when updating a schedule with invalid attributes' do
      patch api_v1_household_schedule_path(household_id, schedules(:john_paracetamol).id),
            params: { schedule: { dose_amount: nil } },
            headers: headers,
            as: :json

      expect(response).to have_http_status(:unprocessable_content)
    end
  end
end
