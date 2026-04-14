# frozen_string_literal: true

FactoryBot.define do
  factory :carer_relationship do
    carer { association :person }
    patient { association :person }
    relationship_type { 'family_member' }
    active { true }
  end
end
