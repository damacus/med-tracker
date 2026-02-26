# frozen_string_literal: true

FactoryBot.define do
  factory :medication do
    sequence(:name) { |n| "Medication #{n}" }
    location
    dosage_amount { 500 }
    dosage_unit { 'mg' }
    current_supply { 50 }
    supply_at_last_restock { 50 }
    expiry_date { 1.year.from_now }
    description { 'Test medication description' }

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
