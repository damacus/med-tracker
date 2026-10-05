# frozen_string_literal: true

class HouseholdExport < ApplicationRecord
  belongs_to :household
  belongs_to :requested_by_account, class_name: 'Account', inverse_of: :requested_household_exports
  has_one_attached :artifact

  enum :status, {
    requested: 'requested',
    generating: 'generating',
    ready: 'ready',
    downloaded: 'downloaded',
    expired: 'expired',
    failed: 'failed'
  }, validate: true

  validates :requested_at, presence: true
end
