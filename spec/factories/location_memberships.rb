# frozen_string_literal: true

FactoryBot.define do
  factory :location_membership do
    location
    person
  end
end
