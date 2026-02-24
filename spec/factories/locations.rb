# frozen_string_literal: true

FactoryBot.define do
  factory :location do
    sequence(:name) { |n| "Location #{n}" }
    description { 'A test location' }

    trait :home do
      name { 'Home' }
      description { 'Primary home location' }
    end

    trait :school do
      name { 'School' }
      description { 'School location' }
    end
  end
end
