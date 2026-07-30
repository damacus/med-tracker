# frozen_string_literal: true

class PushNotificationService
  def self.send_to_account(account, household: Current.household, notification_kind: :unknown, **message)
    return suppress(notification_kind) if household && !household.operational?

    title = message.fetch(:title)
    body = message.fetch(:body)
    path = message.fetch(:path, '/')
    results = send_web_push_to_account(
      account, title:, body:, path:, notification_kind:
    ) + send_native_push_to_account(
      account, title:, body:, path:, notification_kind:
    )
    record_aggregate_result(notification_kind, results)
    results
  end

  def self.suppress(notification_kind)
    Observability::NotificationStage.emit(
      kind: notification_kind,
      stage: :channel_attempt,
      reason: :suppressed
    )
    nil
  end
  private_class_method :suppress

  def self.send_web_push_to_account(account, title:, body:, notification_kind:, path: '/')
    vapid = build_vapid_config
    payload = { title: title, options: { body: body, data: { path: path } } }.to_json

    account.push_subscriptions.map do |sub|
      deliver(sub, payload, vapid, notification_kind:)
    end
  end
  private_class_method :send_web_push_to_account

  def self.send_native_push_to_account(account, title:, body:, notification_kind:, path: '/')
    tokens = account.native_device_tokens

    tokens.map do |token|
      deliver_native(token, title:, body:, path:, notification_kind:)
    end
  end
  private_class_method :send_native_push_to_account

  def self.deliver_native(token, title:, body:, path:, notification_kind:)
    provider = token.platform == 'ios' ? :apns : :fcm
    record_attempt(notification_kind, channel: :native_push, provider:)
    result = native_client_for(token)&.deliver(token, title: title, body: body, path: path)
    return record_provider_result(notification_kind, :delivery_unknown, channel: :native_push, provider:) unless result

    if result.unregistered?
      token.destroy
      return record_provider_result(notification_kind, :permanent_failure, channel: :native_push, provider:)
    end
    if result.status == :delivered
      return record_provider_result(notification_kind, :provider_accepted, channel: :native_push, provider:)
    end

    record_provider_result(notification_kind, :permanent_failure, channel: :native_push, provider:)
  end
  private_class_method :deliver_native

  def self.native_client_for(token)
    case token.platform
    when 'ios'
      NativePush::ApnsClient.new if NativePush::ApnsClient.configured?
    when 'android'
      NativePush::FcmClient.new if NativePush::FcmClient.configured?
    end
  end
  private_class_method :native_client_for

  def self.build_vapid_config
    subject = ENV.fetch('VAPID_SUBJECT',
                        Rails.application.credentials.dig(:vapid, :subject) || 'notifications@example.com')
    {
      subject: "mailto:#{subject}",
      public_key: ENV.fetch('VAPID_PUBLIC_KEY', Rails.application.credentials.dig(:vapid, :public_key)),
      private_key: ENV.fetch('VAPID_PRIVATE_KEY', Rails.application.credentials.dig(:vapid, :private_key))
    }
  end
  private_class_method :build_vapid_config

  def self.deliver(sub, payload, vapid, notification_kind:)
    record_attempt(notification_kind, channel: :web_push, provider: :web_push)
    return reject_web_subscription(notification_kind) unless PushSubscriptionEndpointPolicy.allowed?(sub.endpoint)

    WebPush.payload_send(
      message: payload,
      endpoint: sub.endpoint,
      p256dh: sub.p256dh,
      auth: sub.auth,
      vapid: vapid
    )
    record_provider_result(notification_kind, :provider_accepted, channel: :web_push, provider: :web_push)
  rescue WebPush::ExpiredSubscription, WebPush::InvalidSubscription
    sub.destroy
    record_provider_result(notification_kind, :permanent_failure, channel: :web_push, provider: :web_push)
  rescue StandardError
    record_provider_result(notification_kind, :delivery_unknown, channel: :web_push, provider: :web_push)
  end
  private_class_method :deliver

  def self.reject_web_subscription(notification_kind)
    record_provider_result(
      notification_kind,
      :permanent_failure,
      channel: :web_push,
      provider: :web_push
    )
  end
  private_class_method :reject_web_subscription

  def self.record_attempt(kind, channel:, provider:)
    Observability::NotificationStage.emit(
      kind:,
      stage: :channel_attempt,
      reason: :attempted,
      channel:,
      provider:
    )
  end
  private_class_method :record_attempt

  def self.record_provider_result(kind, reason, channel:, provider:)
    Observability::NotificationStage.emit(
      kind:,
      stage: :provider_outcome,
      reason:,
      channel:,
      provider:
    )
    reason
  end
  private_class_method :record_provider_result

  def self.record_aggregate_result(kind, results)
    accepted = results.count(:provider_accepted)
    return if accepted.zero? || accepted == results.size

    Observability::NotificationStage.emit(
      kind:,
      stage: :provider_outcome,
      reason: :partial_failure,
      channel: :mixed
    )
  end
  private_class_method :record_aggregate_result
end
