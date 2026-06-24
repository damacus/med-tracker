# frozen_string_literal: true

class AccountLockout < ApplicationRecord
  self.primary_key = :account_id

  belongs_to :account, inverse_of: false

  scope :active, -> { where(arel_table[:deadline].gt(Time.current)) }
end
