# frozen_string_literal: true

FactoryBot.define do
  factory :dosage do
    medicine
    amount { 500 }
    unit { 'mg' }
    frequency { 'As needed' }
    description { 'Standard dose' }
  end
end
