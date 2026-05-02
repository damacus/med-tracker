# frozen_string_literal: true

FactoryBot.define do
  factory :account_webauthn_key do
    account
    sequence(:webauthn_id) { |n| "webauthn-id-#{n}" }
    sequence(:public_key) { |n| "public-key-#{n}" }
    sign_count { 0 }
  end
end
