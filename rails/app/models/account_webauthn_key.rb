# frozen_string_literal: true

class AccountWebauthnKey < ApplicationRecord
  belongs_to :account, inverse_of: :account_webauthn_keys

  validates :webauthn_id, presence: true, uniqueness: { scope: :account_id }
  validates :public_key, presence: true
  validates :sign_count, presence: true, numericality: { greater_than_or_equal_to: 0 }
  validates :nickname, presence: true

  before_validation :set_default_nickname, on: :create

  private

  def set_default_nickname
    return if nickname.present?

    index = account.account_webauthn_keys.length + 1
    self.nickname = "Passkey #{index}"
  end
end
