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
  end
end
