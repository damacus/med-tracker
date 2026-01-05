# frozen_string_literal: true

FactoryBot.define do
  factory :user do
    transient do
      name { nil }
    end

    person
    sequence(:email_address) { |n| "user#{n}@example.com" }
    password { 'password' }
    role { :parent }
    active { true }

    after(:build) do |user, evaluator|
      user.person.name = evaluator.name if evaluator.name
    end

    after(:create) do |user|
      if user.person.account.nil?
        account = FactoryBot.create(:account, email: user.email_address)
        user.person.update!(account: account)
      end
    end

    trait :admin do
      role { :administrator }
    end

    trait :doctor do
      role { :doctor }
    end

    trait :nurse do
      role { :nurse }
    end

    trait :carer do
      role { :carer }
    end
  end
end
