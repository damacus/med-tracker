# frozen_string_literal: true

FactoryBot.define do
  factory :medication_take do
    taken_at { Time.current }

    trait :for_person_medication do
      person_medication
      schedule { nil }
    end

    trait :for_schedule do
      schedule
      person_medication { nil }
    end

    trait :recent do
      taken_at { 2.hours.ago }
    end

    trait :today do
      taken_at { Time.current.beginning_of_day + 8.hours }
    end
  end
end
