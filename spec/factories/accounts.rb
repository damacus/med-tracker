# frozen_string_literal: true

FactoryBot.define do
  factory :account do
    sequence(:email) { |n| "account-#{n}@example.com" }
    status { :verified }
  end
end
