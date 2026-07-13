# frozen_string_literal: true

class PushNotificationService
  def self.send_to_account(account, title:, body:, path: '/', household: Current.household)
    return if household && !household.operational?

    send_web_push_to_account(account, title: title, body: body, path: path)
    send_native_push_to_account(account, title: title, body: body, path: path)
  end

  def self.send_web_push_to_account(account, title:, body:, path: '/')
    vapid = build_vapid_config
    payload = { title: title, options: { body: body, data: { path: path } } }.to_json

    account.push_subscriptions.each do |sub|
      deliver(sub, payload, vapid)
    end
  end
  private_class_method :send_web_push_to_account

  def self.send_native_push_to_account(account, title:, body:, path: '/')
    tokens = account.native_device_tokens
    return if tokens.none?

    tokens.each do |token|
      deliver_native(token, title: title, body: body, path: path)
    end
  end
  private_class_method :send_native_push_to_account

  def self.deliver_native(token, title:, body:, path:)
    result = native_client_for(token)&.deliver(token, title: title, body: body, path: path)
    return log_native_skip(token) unless result
    return token.destroy if result.unregistered?
    return if result.status == :delivered

    Rails.logger.error(
      "[PushNotificationService] Native push failed: platform=#{token.platform} " \
      "token_id=#{token.id} status=#{result.provider_status.inspect} error=#{result.provider_error.inspect}"
    )
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

  def self.log_native_skip(token)
    Rails.logger.info(
      "[PushNotificationService] Native push skipped: platform=#{token.platform} token_id=#{token.id}"
    )
  end
  private_class_method :log_native_skip

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

  def self.deliver(sub, payload, vapid)
    unless PushSubscriptionEndpointPolicy.allowed?(sub.endpoint)
      Rails.logger.warn("Skipped unsafe web push endpoint for subscription #{sub.id}")
      return
    end

    WebPush.payload_send(
      message: payload,
      endpoint: sub.endpoint,
      p256dh: sub.p256dh,
      auth: sub.auth,
      vapid: vapid
    )
  rescue WebPush::ExpiredSubscription, WebPush::InvalidSubscription
    sub.destroy
  rescue StandardError => e
    Rails.logger.error("Push notification delivery failed for subscription #{sub.id}: #{e.class}: #{e.message}")
  end
  private_class_method :deliver
end
