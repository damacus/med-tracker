# frozen_string_literal: true

class HouseholdPurgeRun < ApplicationRecord
  belongs_to :household
  belongs_to :requested_by_account, class_name: 'Account'

  enum :status, { pending: 'pending', running: 'running', failed: 'failed', completed: 'completed' }, validate: true

  validates :household_id, uniqueness: true

  class << self
    def acquire!(household:, requested_by_account:)
      household.with_lock do
        find_or_create_by!(household: household) do |run|
          run.requested_by_account = requested_by_account
        end
      end
    end
  end
end
