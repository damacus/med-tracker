# frozen_string_literal: true

class AccountOtpKey < ApplicationRecord
  belongs_to :account, foreign_key: :id, inverse_of: false
end
