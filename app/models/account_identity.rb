# frozen_string_literal: true

# Stores OAuth provider identities linked to accounts
class AccountIdentity < ApplicationRecord
  belongs_to :account

  validates :provider, presence: true
  validates :uid, presence: true, uniqueness: { scope: :provider }
end
