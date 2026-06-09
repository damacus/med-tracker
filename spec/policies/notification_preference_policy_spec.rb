# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NotificationPreferencePolicy, type: :policy do
  fixtures :all

  describe '#show?' do
    it 'permits a clinician to view any preference' do
      expect(described_class.new(users(:doctor), create(:notification_preference)).show?).to be(true)
    end

    it 'permits a user to view their own person preference' do
      user = users(:carer)
      preference = create(:notification_preference, person: user.person)
      expect(described_class.new(user, preference).show?).to be(true)
    end

    it "denies a user viewing another person's preference" do
      expect(described_class.new(users(:carer), create(:notification_preference)).show?).to be(false)
    end

    it 'denies when the user has no person' do
      user_without_person = User.new(role: :carer)
      expect(described_class.new(user_without_person, create(:notification_preference)).show?).to be(false)
    end

    it 'denies when there is no user' do
      expect(described_class.new(nil, create(:notification_preference)).show?).to be(false)
    end
  end

  describe NotificationPreferencePolicy::Scope do
    it 'returns all for a clinician' do
      create(:notification_preference)
      expect(described_class.new(users(:doctor), NotificationPreference.all).resolve).to eq(NotificationPreference.all)
    end

    it 'returns only the user-person rows otherwise' do
      user = users(:carer)
      own = create(:notification_preference, person: user.person)
      create(:notification_preference)
      expect(described_class.new(user, NotificationPreference.all).resolve).to contain_exactly(own)
    end

    it 'returns none when there is no user' do
      expect(described_class.new(nil, NotificationPreference.all).resolve).to be_empty
    end

    it 'returns none when user has no person_id' do
      user_without_person = User.new(role: :carer)
      create(:notification_preference)
      expect(described_class.new(user_without_person, NotificationPreference.all).resolve).to be_empty
    end
  end
end
