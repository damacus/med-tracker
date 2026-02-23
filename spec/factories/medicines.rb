# frozen_string_literal: true

FactoryBot.define do
  factory :medicine do
    sequence(:name) { |n| "Medicine #{n}" }
    location
    dosage_amount { 500 }
    dosage_unit { 'mg' }
    current_supply { 50 }
    stock { 100 }
    expiry_date { 1.year.from_now }
    description { 'Test medicine description' }

    trait :vitamin do
      name { 'Vitamin D' }
      dosage_amount { 1000 }
      dosage_unit { 'IU' }
    end

    trait :painkiller do
      name { 'Paracetamol' }
      dosage_amount { 500 }
      dosage_unit { 'mg' }
    end
  end
end
