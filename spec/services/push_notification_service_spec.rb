# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PushNotificationService do
  fixtures :accounts

  let(:account) { accounts(:admin) }

  describe '.send_to_account' do
    let!(:first_subscription) do
      PushSubscription.create!(
        account: account,
        endpoint: 'https://example.com/push/subscriptions/first',
        p256dh: 'first_public_key',
        auth: 'first_auth_secret'
      )
    end
    let!(:second_subscription) do
      PushSubscription.create!(
        account: account,
        endpoint: 'https://example.com/push/subscriptions/second',
        p256dh: 'second_public_key',
        auth: 'second_auth_secret'
      )
    end

    it 'continues delivering after a transient push failure' do
      calls = 0

      allow(WebPush).to receive(:payload_send) do
        calls += 1
        raise SocketError, 'lookup failed' if calls == 1
      end
      allow(Rails.logger).to receive(:error)

      expect do
        described_class.send_to_account(account, title: 'Medication Reminder', body: 'Take aspirin')
      end.not_to raise_error

      expect(WebPush).to have_received(:payload_send).twice
      expect(Rails.logger).to have_received(:error).with(/Push notification delivery failed/)
    end

    it 'removes expired subscriptions and continues with the rest' do
      calls = 0
      stub_const('WebPush::ExpiredSubscription', Class.new(StandardError))

      allow(WebPush).to receive(:payload_send) do
        calls += 1
        raise WebPush::ExpiredSubscription if calls == 1
      end

      expect do
        described_class.send_to_account(account, title: 'Medication Reminder', body: 'Take aspirin')
      end.to change(PushSubscription, :count).by(-1)

      expect(WebPush).to have_received(:payload_send).twice
      expect(PushSubscription.exists?(first_subscription.id)).to be(false)
      expect(PushSubscription.exists?(second_subscription.id)).to be(true)
    end

    it 'does not write notification title or body content to native push logs' do
      create_native_device_token
      allow(WebPush).to receive(:payload_send)
      allow(Rails.logger).to receive(:info)

      described_class.send_to_account(account, title: 'Medication Reminder', body: 'Take aspirin at 07:15')

      expect(Rails.logger).to have_received(:info) do |message|
        expect(message).to include('Native push skipped')
        expect(message).not_to include('Medication Reminder')
        expect(message).not_to include('Take aspirin')
      end
    end

    it 'delivers native notifications through the platform adapters' do
      ios = create_native_device_token(platform: 'ios', device_token: 'ios-token')
      android = create_native_device_token(platform: 'android', device_token: 'android-token')
      allow(WebPush).to receive(:payload_send)
      apns_client = stub_native_client(NativePush::ApnsClient)
      fcm_client = stub_native_client(NativePush::FcmClient)

      described_class.send_to_account(account, title: 'Medication Reminder', body: 'Take aspirin', path: '/today')

      expect_native_delivery(apns_client, ios)
      expect_native_delivery(fcm_client, android)
    end

    it 'removes native device tokens rejected by providers as unregistered' do
      token = create_native_device_token(platform: 'ios', device_token: 'stale-ios-token')
      allow(WebPush).to receive(:payload_send)
      stub_native_client(NativePush::ApnsClient, result: NativePush::DeliveryResult.unregistered)

      expect do
        described_class.send_to_account(account, title: 'Medication Reminder', body: 'Take aspirin')
      end.to change { NativeDeviceToken.exists?(token.id) }.from(true).to(false)
    end
  end

  def create_native_device_token(platform: 'ios', device_token: 'native-token-for-privacy-test')
    NativeDeviceToken.create!(
      account: account,
      platform: platform,
      device_token: device_token
    )
  end

  def stub_native_client(client_class, result: NativePush::DeliveryResult.delivered)
    client = instance_double(client_class, deliver: result)
    allow(client_class).to receive_messages(configured?: true, new: client)
    client
  end

  def expect_native_delivery(client, token)
    expect(client).to have_received(:deliver).with(
      token,
      title: 'Medication Reminder',
      body: 'Take aspirin',
      path: '/today'
    )
  end
end
