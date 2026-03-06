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
  end
end
