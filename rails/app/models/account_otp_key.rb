# frozen_string_literal: true

class AccountOtpKey < ApplicationRecord
  # Rodauth uses id as both primary key and foreign key to accounts
  self.primary_key = :id

  belongs_to :account, foreign_key: :id, inverse_of: false, optional: true
end
