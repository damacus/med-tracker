# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PushSubscription do
  describe 'associations' do
    it { is_expected.to belong_to(:account) }
  end

  describe 'validations' do
    subject do
      described_class.new(
        account_id: 1,
        endpoint: 'https://example.com/push',
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
  end

  describe '#to_webpush_params' do
    it 'returns a hash with endpoint, p256dh, and auth keys' do
      subscription = described_class.new(
        endpoint: 'https://example.com/push',
        p256dh: 'p256dh_key',
        auth: 'auth_key'
      )

      expect(subscription.to_webpush_params).to eq(
        endpoint: 'https://example.com/push',
        p256dh: 'p256dh_key',
        auth: 'auth_key'
      )
    end
  end
end
