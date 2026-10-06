# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PushNotificationService do
  fixtures :accounts

  let(:account) { accounts(:admin) }
  let(:events) { [] }

  before do
    allow(Observability::CanonicalLogger).to receive(:write) { |event| events << event.to_h }
  end

  describe '.send_to_account' do
    let!(:first_subscription) do
      PushSubscription.create!(
        account: account,
        endpoint: 'https://fcm.googleapis.com/fcm/send/first',
        p256dh: 'first_public_key',
        auth: 'first_auth_secret'
      )
    end
    let!(:second_subscription) do
      PushSubscription.create!(
        account: account,
        endpoint: 'https://fcm.googleapis.com/fcm/send/second',
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

      expect do
        described_class.send_to_account(account, title: 'Medication Reminder', body: 'Take aspirin')
      end.not_to raise_error

      expect(WebPush).to have_received(:payload_send).twice
      expect(notification_reasons).to include('provider_accepted', 'delivery_unknown', 'partial_failure')
    end

    it 'skips unsafe legacy web push endpoints before delivery' do
      PushSubscription.new(
        account: account,
        endpoint: 'https://127.0.0.1/push/internal',
        p256dh: 'legacy_public_key',
        auth: 'legacy_auth_secret'
      ).save!(validate: false)
      allow(WebPush).to receive(:payload_send)

      described_class.send_to_account(account, title: 'Medication Reminder', body: 'Take aspirin')

      expect(WebPush).to have_received(:payload_send).twice
      expect(notification_reasons).to include('permanent_failure')
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

    it 'removes invalid subscriptions and continues with the rest' do
      calls = 0
      stub_const('WebPush::InvalidSubscription', Class.new(StandardError))

      allow(WebPush).to receive(:payload_send) do
        calls += 1
        raise WebPush::InvalidSubscription if calls == 1
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

      described_class.send_to_account(account, title: 'Medication Reminder', body: 'Take aspirin at 07:15')

      expect(events.to_json).not_to include('Medication Reminder', 'Take aspirin', '07:15')
      expect(notification_reasons).to include('delivery_unknown')
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
      expect(notification_reasons.count('provider_accepted')).to eq(4)
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

  describe NativePush::ApnsClient do
    fixtures :accounts

    before do
      allow(described_class).to receive_messages(
        bundle_id: 'test.medtracker', team_id: 'team', key_id: 'key',
        private_key: OpenSSL::PKey::EC.generate('prime256v1').to_pem
      )
    end

    it 'sends private alert text with typed routing metadata' do
      token = NativeDeviceToken.new(account: accounts(:admin), device_token: 'test-token', platform: 'ios')
      request = stub_request(:post, 'https://api.sandbox.push.apple.com/3/device/test-token')
                .with do |req|
        payload = JSON.parse(req.body)
        expect(payload.fetch('aps').fetch('alert')).to eq(
          'title' => 'MedTracker', 'body' => 'Open MedTracker to view your notification.'
        )
        expect(payload).to include('path' => '/households/home/dashboard', 'kind' => 'dose_due')
        expect(req.body).not_to include('Aspirin', 'John')
      end.to_return(status: 200)

      result = described_class.new(environment: 'sandbox').deliver(
        token, title: 'John', body: 'Aspirin', path: '/households/home/dashboard', notification_kind: :dose_due
      )

      expect(result.status).to eq(:delivered)
      expect(request).to have_been_requested.once
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
    routing = token.platform == 'ios' ? { notification_kind: :unknown } : {}
    expect(client).to have_received(:deliver).with(
      token,
      title: 'Medication Reminder',
      body: 'Take aspirin',
      path: '/today',
      **routing
    )
  end

  def notification_reasons
    events.filter_map do |event|
      event['medtracker.reason'] if event['event.name'] == 'notification.stage'
    end
  end
end
