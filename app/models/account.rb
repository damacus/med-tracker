class Account < ApplicationRecord
  include Rodauth::Rails.model
  enum :status, { unverified: 1, verified: 2, closed: 3 }

  # Account can have an associated person (for users with full accounts)
  # A person can exist without an account (minors, dependents)
  has_one :person, dependent: :nullify
  has_many :account_identities, dependent: :destroy

  # Delegate some methods to person if needed
  delegate :name, to: :person, allow_nil: true
end

