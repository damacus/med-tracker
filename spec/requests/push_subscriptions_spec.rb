# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Push subscriptions' do
  fixtures :accounts, :people, :users

  let(:user) { users(:admin) }

  before { sign_in(user) }

  describe 'POST /push_subscription' do
    it 'creates a push subscription for the signed-in account' do
      expect do
        post push_subscription_path,
             params: {
               endpoint: 'https://example.com/push/subscriptions/123',
               keys: {
                 p256dh: 'public_key',
                 auth: 'auth_secret'
               }
             },
             as: :json
      end.to change(PushSubscription, :count).by(1)

      expect(response).to have_http_status(:created)

      subscription = PushSubscription.order(:id).last
      expect(subscription.account).to eq(user.person.account)
      expect(subscription.endpoint).to eq('https://example.com/push/subscriptions/123')
      expect(subscription.p256dh).to eq('public_key')
      expect(subscription.auth).to eq('auth_secret')
    end

    it 'records a redacted creation audit event' do
      expect do
        post push_subscription_path,
             params: {
               endpoint: 'https://example.com/push/subscriptions/123',
               keys: {
                 p256dh: 'public_key',
                 auth: 'auth_secret'
               }
             },
             as: :json
      end.to change {
        PaperTrail::Version.where(item_type: 'AuthenticationToken',
                                  event: 'auth_token/push_subscription/created').count
      }.by(1)

      version = PaperTrail::Version.where(item_type: 'AuthenticationToken').last
      data = JSON.parse(version.object)

      expect(version.item_id).to eq(user.person.account.id)
      expect(data['endpoint_hash']).to eq(Digest::SHA256.hexdigest('https://example.com/push/subscriptions/123'))
      expect(version.object).not_to include('https://example.com/push/subscriptions/123')
      expect(version.object).not_to include('public_key')
      expect(version.object).not_to include('auth_secret')
    end

    it 'returns a validation error when required keys are missing' do
      post push_subscription_path,
           params: {
             endpoint: 'https://example.com/push/subscriptions/missing-keys',
             keys: {
               p256dh: '',
               auth: ''
             }
           },
           as: :json

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.parsed_body['error']).to eq('Unable to save push subscription.')
      expect(response.parsed_body['errors']).to include("P256dh can't be blank", "Auth can't be blank")
    end

    it 'returns a validation error instead of crashing when the endpoint is already taken' do
      PushSubscription.create!(
        account: users(:jane).person.account,
        endpoint: 'https://example.com/push/subscriptions/shared',
        p256dh: 'existing_public_key',
        auth: 'existing_auth_secret'
      )

      post push_subscription_path,
           params: {
             endpoint: 'https://example.com/push/subscriptions/shared',
             keys: {
               p256dh: 'public_key',
               auth: 'auth_secret'
             }
           },
           as: :json

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.parsed_body['error']).to eq('Unable to save push subscription.')
      expect(response.parsed_body['errors']).to include('Endpoint has already been taken')
    end
  end

  describe 'DELETE /push_subscription' do
    it 'returns no content when the endpoint does not exist' do
      delete push_subscription_path,
             params: { endpoint: 'https://example.com/push/subscriptions/missing' },
             as: :json

      expect(response).to have_http_status(:no_content)
    end

    it 'records a redacted revocation audit event' do
      PushSubscription.create!(
        account: user.person.account,
        endpoint: 'https://example.com/push/subscriptions/123',
        p256dh: 'public_key',
        auth: 'auth_secret'
      )

      expect do
        delete push_subscription_path,
               params: { endpoint: 'https://example.com/push/subscriptions/123' },
               as: :json
      end.to change {
        PaperTrail::Version.where(item_type: 'AuthenticationToken',
                                  event: 'auth_token/push_subscription/revoked').count
      }.by(1)

      version = PaperTrail::Version.where(item_type: 'AuthenticationToken').last
      expect(version.object).not_to include('https://example.com/push/subscriptions/123')
      expect(version.object).not_to include('public_key')
      expect(version.object).not_to include('auth_secret')
    end
  end

  describe 'POST /push_subscription/test' do
    it 'sends a test notification for the signed-in account' do
      allow(PushNotificationService).to receive(:send_to_account)

      post test_push_subscription_path, as: :json

      expect(response).to have_http_status(:no_content)
      expect(PushNotificationService).to have_received(:send_to_account).with(
        user.person.account,
        title: 'MedTracker Test',
        body: 'Push notifications are working correctly from the server.'
      )
    end

    it 'returns a JSON error when the test notification cannot be sent' do
      allow(PushNotificationService).to receive(:send_to_account).and_raise(SocketError, 'lookup failed')
      allow(Rails.logger).to receive(:error)

      post test_push_subscription_path, as: :json

      expect(response).to have_http_status(:service_unavailable)
      expect(response.parsed_body['error']).to eq('Unable to send test notification.')
      expect(Rails.logger).to have_received(:error).with(/Test push notification failed/)
    end
  end
end
