# frozen_string_literal: true

class NativeDeviceToken < ApplicationRecord
  PLATFORMS = %w[ios android].freeze

  belongs_to :account

  validates :device_token, presence: true, uniqueness: true
  validates :platform, inclusion: { in: PLATFORMS }
  validates :apns_environment, inclusion: { in: %w[sandbox production] }, allow_nil: true
end
