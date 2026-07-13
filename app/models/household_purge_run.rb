# frozen_string_literal: true

class HouseholdPurgeRun < ApplicationRecord
  belongs_to :household
  belongs_to :requested_by_account, class_name: 'Account'

  enum :status, { pending: 'pending', running: 'running', failed: 'failed', completed: 'completed' }, validate: true
end
