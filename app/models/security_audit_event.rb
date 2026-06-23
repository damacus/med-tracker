# frozen_string_literal: true

class SecurityAuditEvent < ApplicationRecord
  belongs_to :household, optional: true
  belongs_to :actor_account, class_name: 'Account', optional: true
  belongs_to :actor_membership, class_name: 'HouseholdMembership', optional: true

  validates :event_type, presence: true
end
