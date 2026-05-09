# frozen_string_literal: true

class Account < ApplicationRecord
  include Rodauth::Rails.model

  has_paper_trail skip: %i[password_hash]

  enum :status, { unverified: 1, verified: 2, closed: 3 }
  enum :subscription_plan, { free: 'free', family_plus: 'family_plus' }, validate: true

  has_one :person, dependent: :nullify
  has_many :api_sessions, dependent: :destroy
  has_many :push_subscriptions, dependent: :destroy
  has_many :native_device_tokens, dependent: :destroy
  has_many :account_webauthn_keys, dependent: :destroy
  has_many :account_webauthn_user_ids, dependent: :destroy

  validates :email, presence: true, uniqueness: true
end
