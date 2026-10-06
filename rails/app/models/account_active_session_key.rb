# frozen_string_literal: true

class AccountActiveSessionKey < ApplicationRecord
  self.primary_key = %i[account_id session_id]

  belongs_to :account
end
