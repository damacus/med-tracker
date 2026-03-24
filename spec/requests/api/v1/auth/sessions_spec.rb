# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 auth sessions' do
  fixtures :accounts, :people, :users, :locations, :location_memberships

  let(:user) { users(:jane) }

  describe 'POST /api/v1/auth/login' do
    it 'returns access and refresh tokens with the current user payload' do
      post api_v1_auth_login_path,
           params: {
             email: user.email_address,
             password: 'password',
             device_name: 'RSpec iPhone'
           },
           as: :json

      expect(response).to have_http_status(:created)
      expect(response.parsed_body.dig('data', 'access_token')).to be_present
      expect(response.parsed_body.dig('data', 'refresh_token')).to be_present
      expect(response.parsed_body.dig('data', 'me', 'email_address')).to eq(user.email_address)

      api_session = ApiSession.order(:id).last
      expect(api_session.account).to eq(user.person.account)
      expect(api_session.device_name).to eq('RSpec iPhone')
    end

    it 'rejects an invalid password' do
      post api_v1_auth_login_path,
           params: {
             email: user.email_address,
             password: 'wrong-password'
           },
           as: :json

      expect(response).to have_http_status(:unauthorized)
      expect(response.parsed_body.dig('error', 'code')).to eq('invalid_credentials')
    end
  end

  describe 'POST /api/v1/auth/refresh' do
    it 'rotates refresh tokens and invalidates the old refresh token' do
      login_data = api_login(user)

      post api_v1_auth_refresh_path,
           params: { refresh_token: login_data.fetch('refresh_token') },
           as: :json

      expect(response).to have_http_status(:ok)
      new_refresh_token = response.parsed_body.dig('data', 'refresh_token')
      expect(new_refresh_token).to be_present
      expect(new_refresh_token).not_to eq(login_data.fetch('refresh_token'))

      post api_v1_auth_refresh_path,
           params: { refresh_token: login_data.fetch('refresh_token') },
           as: :json

      expect(response).to have_http_status(:unauthorized)
      expect(response.parsed_body.dig('error', 'code')).to eq('invalid_refresh_token')
    end
  end

  describe 'DELETE /api/v1/auth/logout' do
    it 'revokes the current access token' do
      login_data = api_login(user)
      headers = api_auth_headers(login_data.fetch('access_token'))

      delete api_v1_auth_logout_path, headers: headers, as: :json

      expect(response).to have_http_status(:no_content)

      get api_v1_me_path, headers: headers, as: :json

      expect(response).to have_http_status(:unauthorized)
      expect(response.parsed_body.dig('error', 'code')).to eq('unauthorized')
    end
  end
end
