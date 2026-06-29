# frozen_string_literal: true

FactoryBot.define do
  factory :household do
    sequence(:name) { |n| "Household #{n}" }
    sequence(:slug) { |n| "household-#{n}" }
    status { :active }
    timezone { 'UTC' }
    subscription_plan { :free }
  end
end
