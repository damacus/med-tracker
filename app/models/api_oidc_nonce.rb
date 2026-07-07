# frozen_string_literal: true

class ApiOidcNonce < ApplicationRecord
  validates :issuer, :subject, :nonce, :used_at, presence: true
  validates :nonce, uniqueness: { scope: %i[issuer subject] }
end
