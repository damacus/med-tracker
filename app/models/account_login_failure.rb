# frozen_string_literal: true

class AccountLoginFailure < ApplicationRecord
  self.primary_key = :account_id

  belongs_to :account, inverse_of: false
end
