# frozen_string_literal: true

FactoryBot.define do
  factory :household_invitation do
    transient do
      owner_email { "household-inviter-#{SecureRandom.hex(6)}@example.test" }
      household_name { "Household Invitation #{SecureRandom.hex(4)}" }
    end

    household do
      account = Account.create!(email: owner_email, status: :verified)
      Household.create_with_owner!(
        name: household_name,
        owner_account: account,
        owner_person_attributes: {
          name: "#{household_name} Owner",
          date_of_birth: 30.years.ago.to_date,
          person_type: :adult,
          has_capacity: true
        }
      )
    end
    invited_by_membership { household.household_memberships.owner.sole }
    sequence(:email) { |n| "household-invitee#{n}@example.com" }
    membership_role { :member }
    accepted_at { nil }

    trait :accepted do
      accepted_at { Time.current }
    end

    trait :expired do
      expires_at { 1.day.ago }
    end
  end
end
