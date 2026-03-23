# frozen_string_literal: true

class NativeDeviceToken < ApplicationRecord
  PLATFORMS = %w[ios android].freeze

  belongs_to :account

  validates :device_token, presence: true, uniqueness: true
  validates :platform, inclusion: { in: PLATFORMS }
end
