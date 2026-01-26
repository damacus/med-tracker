# frozen_string_literal: true

FactoryBot.define do
  factory :account do
    sequence(:email) { |n| "user#{n}@example.com" }
    password { 'password123' }
    password_hash { BCrypt::Password.create('password123') }
    status { :verified }
  end
end
