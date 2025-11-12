# frozen_string_literal: true

FactoryBot.define do
  factory :person do
    sequence(:name) { |n| "Person #{n}" }
    date_of_birth { 30.years.ago }
    person_type { :adult }
    has_capacity { true }

    trait :minor do
      date_of_birth { 10.years.ago }
      person_type { :minor }
      has_capacity { false }
    end

    trait :dependent_adult do
      date_of_birth { 40.years.ago }
      person_type { :dependent_adult }
      has_capacity { false }
    end
  end
end
