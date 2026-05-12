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

    it 'records a redacted session creation audit event' do
      expect do
        post api_v1_auth_login_path,
             params: {
               email: user.email_address,
               password: 'password',
               device_name: 'RSpec iPhone'
             },
             as: :json
      end.to change {
        PaperTrail::Version.where(item_type: 'AuthenticationToken',
                                  event: 'auth_token/api_session/created').count
      }.by(1)

      access_token = response.parsed_body.dig('data', 'access_token')
      refresh_token = response.parsed_body.dig('data', 'refresh_token')
      version = PaperTrail::Version.where(item_type: 'AuthenticationToken').last
      data = JSON.parse(version.object)

      expect(version.item_id).to eq(user.person.account.id)
      expect(version.whodunnit).to eq(user.id.to_s)
      expect(data['device_name_present']).to be true
      expect(version.object).not_to include(access_token)
      expect(version.object).not_to include(refresh_token)
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

    it 'records a redacted rotation audit event' do
      login_data = api_login(user)

      expect do
        post api_v1_auth_refresh_path,
             params: { refresh_token: login_data.fetch('refresh_token') },
             as: :json
      end.to change {
        PaperTrail::Version.where(item_type: 'AuthenticationToken',
                                  event: 'auth_token/api_session/rotated').count
      }.by(1)

      version = PaperTrail::Version.where(item_type: 'AuthenticationToken').last
      expect(version.item_id).to eq(user.person.account.id)
      expect(version.object).not_to include(response.parsed_body.dig('data', 'access_token'))
      expect(version.object).not_to include(response.parsed_body.dig('data', 'refresh_token'))
    end

    it 'records an expired audit event for a known expired refresh token' do
      login_data = api_login(user)
      session = ApiSession.lookup_by_refresh_token(login_data.fetch('refresh_token'))
      session.update!(refresh_expires_at: 1.minute.ago)

      expect do
        post api_v1_auth_refresh_path,
             params: { refresh_token: login_data.fetch('refresh_token') },
             as: :json
      end.to change {
        PaperTrail::Version.where(item_type: 'AuthenticationToken',
                                  event: 'auth_token/api_session/expired').count
      }.by(1)

      expect(response).to have_http_status(:unauthorized)
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

    it 'records a revoked audit event' do
      login_data = api_login(user)
      headers = api_auth_headers(login_data.fetch('access_token'))

      expect do
        delete api_v1_auth_logout_path, headers: headers, as: :json
      end.to change {
        PaperTrail::Version.where(item_type: 'AuthenticationToken',
                                  event: 'auth_token/api_session/revoked').count
      }.by(1)
    end
  end
end
