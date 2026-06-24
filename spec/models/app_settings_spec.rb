# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AppSettings do
  describe '.instance' do
    before { described_class.delete_all }

    it 'defaults invite-only mode on when a household owner already exists' do
      create_household_with_owner

      expect(described_class.instance).to be_invite_only
    end

    it 'defaults invite-only mode off when no household owner exists' do
      HouseholdMembership.owner.delete_all

      expect(described_class.instance).not_to be_invite_only
    end

    it 'honors an explicit INVITE_ONLY override when creating the settings row' do
      create_household_with_owner(email: 'env-owner@example.test', name: 'Env Family')

      original_invite_only = ENV.fetch('INVITE_ONLY', nil)
      ENV['INVITE_ONLY'] = 'false'

      expect(described_class.instance).not_to be_invite_only
    ensure
      if original_invite_only.nil?
        ENV.delete('INVITE_ONLY')
      else
        ENV['INVITE_ONLY'] = original_invite_only
      end
    end
  end

  def create_household_with_owner(email: 'settings-owner@example.test', name: 'Settings Family')
    account = Account.create!(email: email, status: :verified)
    Household.create_with_owner!(
      name: name,
      owner_account: account,
      owner_person_attributes: {
        name: 'Owner',
        date_of_birth: 30.years.ago.to_date,
        person_type: :adult,
        has_capacity: true
      }
    )
  end

  describe 'versioning' do
    it 'creates a version when invite-only mode changes' do
      settings = described_class.instance

      expect do
        settings.update!(invite_only: !settings.invite_only)
      end.to change { PaperTrail::Version.where(item_type: 'AppSettings', item_id: settings.id).count }.by(1)

      expect(PaperTrail::Version.where(item_type: 'AppSettings', item_id: settings.id).last.event).to eq('update')
    end
  end
end
