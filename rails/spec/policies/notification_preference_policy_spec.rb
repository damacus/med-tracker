# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NotificationPreferencePolicy, type: :policy do
  let(:member) { household_policy_member(role: :member) }
  let(:household) { member.fetch(:household) }
  let(:person) { household_policy_person(household) }
  let(:preference) { household.notification_preferences.create!(person: person) }
  let(:other_preference) do
    other_household = household_policy_member(role: :owner).fetch(:household)
    other_household.notification_preferences.create!(person: household_policy_person(other_household))
  end

  it 'shows preferences through explicit view grants' do
    expect(described_class.new(member.fetch(:context), preference).show?).to be(false)

    grant_policy_person_access(member, person, access_level: :view)

    expect(described_class.new(member.fetch(:context), preference).show?).to be(true)
    expect(described_class.new(member.fetch(:context), other_preference).show?).to be(false)
  end

  it 'scopes preferences to granted people in the household' do
    preference
    other_preference
    grant_policy_person_access(member, person, access_level: :view)

    resolved = described_class::Scope.new(member.fetch(:context), NotificationPreference.all).resolve

    expect(resolved).to contain_exactly(preference)
  end
end
