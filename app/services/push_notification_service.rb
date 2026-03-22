# frozen_string_literal: true

class PushNotificationService
  def self.send_to_account(account, title:, body:, path: '/')
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

  # Delivers OS push notifications to registered native iOS/Android devices.
  #
  # iOS uses APNs (Apple Push Notification service) and Android uses FCM
  # (Firebase Cloud Messaging). This method logs the intent; wire up your
  # preferred APNs/FCM gem (e.g. houston, apnotic, fcm) when those
  # credentials are available.
  #
  # Suggested gems:
  #   iOS  — apnotic (https://github.com/ostinelli/apnotic)
  #   Android — fcm (https://github.com/spacialdb/fcm)
  def self.send_native_push_to_account(account, title:, body:, path: '/')
    tokens = account.native_device_tokens
    return if tokens.none?

    tokens.each do |token|
      Rails.logger.info(
        "[PushNotificationService] Native push queued: platform=#{token.platform} " \
        "token=#{token.device_token.first(8)}… title=#{title.inspect} path=#{path.inspect}"
      )
      # TODO: deliver via APNs (platform "ios") or FCM (platform "android")
    end
  end
  private_class_method :send_native_push_to_account

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
