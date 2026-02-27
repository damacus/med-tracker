# frozen_string_literal: true

class PushNotificationService
  def self.send_to_account(account, title:, body:, path: '/')
    vapid = build_vapid_config
    payload = { title: title, options: { body: body, data: { path: path } } }.to_json

    account.push_subscriptions.each do |sub|
      deliver(sub, payload, vapid)
    end
  end

  def self.build_vapid_config
    {
      subject: "mailto:#{Rails.application.credentials.dig(:vapid, :subject) || 'notifications@example.com'}",
      public_key: Rails.application.credentials.dig(:vapid, :public_key),
      private_key: Rails.application.credentials.dig(:vapid, :private_key)
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
  end
  private_class_method :deliver
end
