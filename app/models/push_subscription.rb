# frozen_string_literal: true

class PushSubscription < ApplicationRecord
  belongs_to :account

  validates :endpoint, presence: true, uniqueness: true
  validates :p256dh, presence: true
  validates :auth, presence: true

  def to_webpush_params
    { endpoint: endpoint, p256dh: p256dh, auth: auth }
  end
end
