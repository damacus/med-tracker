# frozen_string_literal: true

FactoryBot.define do
  factory :account do
    sequence(:email) { |n| "account#{n}@example.com" }
    status { :verified }

    # Rodauth uses BCrypt by default
    password_hash { BCrypt::Password.create('password') }

    trait :unverified do
      status { :unverified }
    end

    trait :closed do
      status { :closed }
    end
  end
end
