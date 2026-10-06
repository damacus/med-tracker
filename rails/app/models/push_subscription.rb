# frozen_string_literal: true

class PushSubscription < ApplicationRecord
  belongs_to :account

  validates :endpoint, presence: true, uniqueness: true
  validates :p256dh, presence: true
  validates :auth, presence: true
  validate :endpoint_must_be_supported_push_service

  def to_webpush_params
    { endpoint: endpoint, p256dh: p256dh, auth: auth }
  end

  private

  def endpoint_must_be_supported_push_service
    return if endpoint.blank? || PushSubscriptionEndpointPolicy.allowed?(endpoint)

    errors.add(:endpoint, 'must be a supported HTTPS Web Push endpoint')
  end
end
