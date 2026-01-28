# frozen_string_literal: true

FactoryBot.define do
  factory :account do
    sequence(:email) { |n| "user#{n}@example.com" }
    password { 'password1234' }
    password_hash { BCrypt::Password.create('password1234') }
    status { :verified }
  end
end
