# frozen_string_literal: true

class AccountRecoveryCode < ApplicationRecord
  self.primary_key = %i[id code]

  belongs_to :account, foreign_key: :id, inverse_of: false
end
