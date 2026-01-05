# frozen_string_literal: true

FactoryBot.define do
  factory :invitation do
    sequence(:email) { |n| "invitee#{n}@example.com" }
    role { :parent }
    token { SecureRandom.urlsafe_base64(32) }
    expires_at { 7.days.from_now }
    accepted_at { nil }

    trait :accepted do
      accepted_at { Time.current }
    end

    trait :expired do
      expires_at { 1.day.ago }
    end
  end
end
