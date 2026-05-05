# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Notification preferences turbo streams' do
  fixtures :accounts, :people, :users

  let(:user) { users(:damacus) }

  before { sign_in(user) }

  describe 'PATCH /notification_preference' do
    it 'returns turbo_stream and replaces the notifications card and flash' do
      patch notification_preference_path,
            params: {
              notification_preference: {
                enabled: '0',
                morning_time: '07:30',
                afternoon_time: '13:30',
                evening_time: '18:30',
                night_time: '22:30'
              }
            },
            headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include('target="notifications-card"')
      expect(response.body).to include('target="flash"')
      expect(user.person.notification_preference.reload).not_to be_enabled
    end
  end
end
