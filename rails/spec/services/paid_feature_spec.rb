# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PaidFeature do
  fixtures :accounts, :people, :users

  describe '.enabled?' do
    it 'keeps AI medication help disabled by default' do
      allow(ENV).to receive(:fetch).with('MEDTRACKER_AI_MEDICATION_HELP_ENABLED', 'false').and_return('false')

      expect(described_class.enabled?(:ai_medication_help, user: users(:admin))).to be(false)
    end

    it 'keeps AI medication help disabled for free households when the environment flag is true' do
      Current.household = Household.create!(name: 'Free Household', slug: 'free-household', subscription_plan: :free)
      allow(ENV).to receive(:fetch).with('MEDTRACKER_AI_MEDICATION_HELP_ENABLED', 'false').and_return('true')

      expect(described_class.enabled?(:ai_medication_help, user: users(:admin))).to be(false)
    ensure
      Current.reset
    end

    it 'enables AI medication help when the environment flag is true and the household plan includes it' do
      Current.household = Household.create!(name: 'Paid Household', slug: 'paid-household',
                                            subscription_plan: :family_plus)
      allow(ENV).to receive(:fetch).with('MEDTRACKER_AI_MEDICATION_HELP_ENABLED', 'false').and_return('true')

      expect(described_class.enabled?(:ai_medication_help, user: users(:admin))).to be(true)
    ensure
      Current.reset
    end

    it 'uses the user membership household when no current household is set' do
      person = users(:admin).person
      household = person.household
      household.update!(subscription_plan: :family_plus)
      household.household_memberships.find_or_create_by!(account: person.account, person: person) do |membership|
        membership.role = :owner
        membership.status = :active
      end
      Current.household = nil
      allow(ENV).to receive(:fetch).with('MEDTRACKER_AI_MEDICATION_HELP_ENABLED', 'false').and_return('true')

      expect(described_class.enabled?(:ai_medication_help, user: users(:admin))).to be(true)
    end

    it 'uses the account active household plan when the membership points at the user tenant' do
      person = users(:admin).person
      household = person.household
      household.update!(subscription_plan: :family_plus)
      household.household_memberships.find_or_create_by!(account: person.account) do |membership|
        membership.role = :owner
        membership.status = :active
      end
      Current.household = nil
      allow(ENV).to receive(:fetch).with('MEDTRACKER_AI_MEDICATION_HELP_ENABLED', 'false').and_return('true')

      expect(described_class.enabled?(:ai_medication_help, user: users(:admin))).to be(true)
    end

    it 'keeps AI medication help disabled without a resolvable household' do
      allow(ENV).to receive(:fetch).with('MEDTRACKER_AI_MEDICATION_HELP_ENABLED', 'false').and_return('true')

      expect(described_class.enabled?(:ai_medication_help, user: nil)).to be(false)
    end

    it 'keeps AI medication help disabled for entitled households when the environment flag is false' do
      Current.household = Household.create!(name: 'Flag Disabled Household', slug: 'flag-disabled-household',
                                            subscription_plan: :family_plus)
      allow(ENV).to receive(:fetch).with('MEDTRACKER_AI_MEDICATION_HELP_ENABLED', 'false').and_return('false')

      expect(described_class.enabled?(:ai_medication_help, user: users(:admin))).to be(false)
    ensure
      Current.reset
    end

    it 'disables unknown paid features' do
      expect(described_class.enabled?(:unknown, user: users(:admin))).to be(false)
    end
  end
end
