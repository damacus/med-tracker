# frozen_string_literal: true

FactoryBot.define do
  factory :person_medicine do
    person
    medicine
    notes { nil }
    max_daily_doses { nil }
    min_hours_between_doses { nil }

    trait :with_max_doses do
      max_daily_doses { 3 }
    end

    trait :with_min_hours do
      min_hours_between_doses { 4 }
    end

    trait :with_both_restrictions do
      max_daily_doses { 2 }
      min_hours_between_doses { 12 }
    end
  end
end
