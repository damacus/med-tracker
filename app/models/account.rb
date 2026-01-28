# frozen_string_literal: true

class Account < ApplicationRecord
  include Rodauth::Rails.model

  enum :status, { unverified: 1, verified: 2, closed: 3 }

  has_one :person, dependent: :nullify
  has_many :account_webauthn_keys, dependent: :destroy
  has_many :account_webauthn_user_ids, dependent: :destroy

  validates :email, presence: true, uniqueness: true
end
