# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PushSubscription do
  fixtures :accounts

  describe 'associations' do
    it { is_expected.to belong_to(:account) }
  end

  describe 'validations' do
    subject do
      described_class.new(
        account_id: 1,
        endpoint: 'https://fcm.googleapis.com/fcm/send/registration-token',
        p256dh: 'p256dh_key',
        auth: 'auth_key'
      )
    end

    it { is_expected.to validate_presence_of(:endpoint) }

    # We use a dummy subject in memory to avoid uniqueness DB insertion requirements
    # that usually require building and saving an account object.
    it { is_expected.to validate_uniqueness_of(:endpoint) }

    it { is_expected.to validate_presence_of(:p256dh) }
    it { is_expected.to validate_presence_of(:auth) }

    it 'allows known browser Web Push service endpoints' do
      expect(valid_subscription(endpoint: 'https://fcm.googleapis.com/fcm/send/registration-token')).to be_valid
      expect(valid_subscription(endpoint: 'https://updates.push.services.mozilla.com/wpush/v2/registration-token'))
        .to be_valid
      expect(valid_subscription(endpoint: 'https://web.push.apple.com/registration-token')).to be_valid
    end

    it 'rejects non-HTTPS and local network endpoints' do
      endpoints = [
        'http://fcm.googleapis.com/fcm/send/registration-token',
        'https://localhost/push',
        'https://127.0.0.1/push',
        'https://10.0.0.5/push',
        'https://169.254.169.254/latest/meta-data',
        'https://example.com/push'
      ]

      endpoints.each do |endpoint|
        subscription = valid_subscription(endpoint: endpoint)

        expect(subscription).not_to be_valid
        expect(subscription.errors[:endpoint]).to include('must be a supported HTTPS Web Push endpoint')
      end
    end
  end

  describe '#to_webpush_params' do
    it 'returns a hash with endpoint, p256dh, and auth keys' do
      subscription = described_class.new(
        endpoint: 'https://fcm.googleapis.com/fcm/send/registration-token',
        p256dh: 'p256dh_key',
        auth: 'auth_key'
      )

      expect(subscription.to_webpush_params).to eq(
        endpoint: 'https://fcm.googleapis.com/fcm/send/registration-token',
        p256dh: 'p256dh_key',
        auth: 'auth_key'
      )
    end
  end

  def valid_subscription(endpoint:)
    described_class.new(
      account: accounts(:admin),
      endpoint: endpoint,
      p256dh: 'p256dh_key',
      auth: 'auth_key'
    )
  end
end
