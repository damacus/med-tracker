# frozen_string_literal: true

FactoryBot.define do
  factory :location_membership do
    household { location.household }
    location { association(:location, household: Current.household || Household.first || association(:household)) }
    person { association(:person, household: location.household) }
  end
end
