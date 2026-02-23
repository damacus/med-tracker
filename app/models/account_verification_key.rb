# frozen_string_literal: true

class AccountVerificationKey < ApplicationRecord
  self.primary_key = nil

  belongs_to :account, inverse_of: false
end
