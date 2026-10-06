# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 notification preferences' do
  fixtures :accounts, :people, :users, :households

  let(:user) { users(:admin) }
  let(:headers) { api_auth_headers(api_login(user).fetch('access_token')) }
  let(:household_id) { api_login(user).dig('household', 'id') }

  describe 'GET /api/v1/households/:household_id/notification_preference' do
    it 'shows the current notification preference' do
      create(:notification_preference, person: user.person, enabled: true)

      get api_v1_household_notification_preference_path(household_id),
          headers: headers,
          as: :json

      expect(response).to have_http_status(:ok)
      expect(response.parsed_body.dig('data', 'person_id')).to eq(user.person_id)
    end

    it 'returns the signed-in users notification preference' do
      people(:admin).create_notification_preference!(
        enabled: true,
        morning_time: '08:00',
        afternoon_time: '14:00',
        evening_time: '18:00',
        night_time: '22:00'
      )

      get api_v1_household_notification_preference_path(household_id),
          headers: headers,
          as: :json

      expect(response).to have_http_status(:ok)
      expect(response.parsed_body.dig('data', 'person_id')).to eq(people(:admin).id)
      expect(response.parsed_body.dig('data', 'morning_time')).to eq('08:00:00')
    end

    it 'returns not found when no preference exists' do
      user.person.notification_preference&.destroy!

      get api_v1_household_notification_preference_path(household_id),
          headers: headers,
          as: :json

      expect(response).to have_http_status(:not_found)
    end
  end

  describe 'PATCH /api/v1/households/:household_id/notification_preference' do
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
  end
end
