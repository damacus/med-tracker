# frozen_string_literal: true

class AccountWebauthnUserId < ApplicationRecord
  belongs_to :account

  validates :webauthn_id, presence: true, uniqueness: true
end
