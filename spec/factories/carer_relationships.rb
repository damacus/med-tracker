# frozen_string_literal: true

FactoryBot.define do
  factory :carer_relationship do
    household { nil }
    patient { association :person, household: household || association(:household) }
    carer { association :person, household: household || patient.household }
    relationship_type { 'family_member' }
    active { true }
  end
end
