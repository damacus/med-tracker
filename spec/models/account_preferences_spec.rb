# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Account do
  fixtures :accounts

  describe '#active_household_membership_for' do
    it 'returns nil when no household is supplied' do
      expect(accounts(:damacus).active_household_membership_for(nil)).to be_nil
    end

    it 'returns the active membership for the supplied household' do
      account = accounts(:damacus)
      household = Household.create!(name: 'Preference Household', slug: 'preference-household')
      membership = household.household_memberships.create!(
        account: account,
        role: :member,
        status: :active
      )

      expect(account.active_household_membership_for(household)).to eq(membership)
    end
  end

  describe '#wizard_variant' do
    it 'defaults to fullpage' do
      expect(accounts(:damacus).wizard_variant).to eq('fullpage')
    end

    it 'keeps a valid preference value' do
      account = accounts(:damacus)

      account.wizard_variant = 'modal'

      expect(account.wizard_variant).to eq('modal')
    end

    it 'falls back to fullpage for unknown preference values' do
      account = accounts(:damacus)

      account.wizard_variant = 'unknown'

      expect(account.wizard_variant).to eq('fullpage')
    end
  end

  describe '#dashboard_variant' do
    it 'defaults to the current dashboard' do
      expect(accounts(:damacus).dashboard_variant).to eq('current')
    end

    it 'keeps a valid preference value' do
      account = accounts(:damacus)

      account.dashboard_variant = 'time_first'

      expect(account.dashboard_variant).to eq('time_first')
    end

    it 'falls back to the current dashboard for unknown preference values' do
      account = accounts(:damacus)

      account.dashboard_variant = 'unknown'

      expect(account.dashboard_variant).to eq('current')
    end
  end

  describe '#gravatar_enabled?' do
    it 'defaults to false' do
      expect(accounts(:damacus).gravatar_enabled?).to be(false)
    end

    it 'normalizes truthy preference values' do
      account = accounts(:damacus)

      account.gravatar_enabled = '1'

      expect(account.gravatar_enabled?).to be(true)
    end

    it 'normalizes falsey preference values' do
      account = accounts(:damacus)

      account.gravatar_enabled = 'false'

      expect(account.gravatar_enabled?).to be(false)
    end
  end
end
