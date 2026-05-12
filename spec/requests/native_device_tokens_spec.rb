# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Native device tokens' do
  fixtures :accounts, :people, :users

  let(:user) { users(:admin) }
  let(:other_account) { users(:jane).person.account }

  before { sign_in(user) }

  describe 'POST /native_device_tokens' do
    it 'creates a token for the signed-in account' do
      expect do
        post native_device_tokens_path,
             params: { device_token: 'abc123', platform: 'ios' },
             as: :json
      end.to change(NativeDeviceToken, :count).by(1)

      expect(response).to have_http_status(:created)

      token = NativeDeviceToken.order(:id).last
      expect(token.account).to eq(user.person.account)
      expect(token.device_token).to eq('abc123')
      expect(token.platform).to eq('ios')
    end

    it 'records a redacted creation audit event' do
      expect do
        post native_device_tokens_path,
             params: { device_token: 'abc123', platform: 'ios' },
             as: :json
      end.to change {
        PaperTrail::Version.where(item_type: 'AuthenticationToken',
                                  event: 'auth_token/native_device_token/created').count
      }.by(1)

      version = PaperTrail::Version.where(item_type: 'AuthenticationToken').last
      data = JSON.parse(version.object)

      expect(version.item_id).to eq(user.person.account.id)
      expect(data['platform']).to eq('ios')
      expect(version.object).not_to include('abc123')
    end

    it 'refuses to hijack a token already owned by another account' do
      existing = NativeDeviceToken.create!(
        account: other_account,
        device_token: 'victim-token',
        platform: 'ios'
      )

      expect do
        post native_device_tokens_path,
             params: { device_token: 'victim-token', platform: 'android' },
             as: :json
      end.not_to(change { existing.reload.account_id })

      expect(response).to have_http_status(:unprocessable_content)
      expect(existing.reload.platform).to eq('ios')
    end
  end

  describe 'DELETE /native_device_tokens/:id' do
    it 'records a redacted revocation audit event' do
      NativeDeviceToken.create!(account: user.person.account, device_token: 'abc123', platform: 'ios')

      expect do
        delete native_device_token_path('abc123'), as: :json
      end.to change {
        PaperTrail::Version.where(item_type: 'AuthenticationToken',
                                  event: 'auth_token/native_device_token/revoked').count
      }.by(1)

      expect(PaperTrail::Version.where(item_type: 'AuthenticationToken').last.object).not_to include('abc123')
    end
  end
end
