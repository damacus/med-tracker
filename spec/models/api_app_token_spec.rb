# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ApiAppToken do
  fixtures :accounts

  describe 'expiry' do
    let(:account) { accounts(:jane_doe) }
    let(:membership) { api_membership_for(account) }
    let(:issued_at) { Time.zone.parse('2024-02-29 10:00:00') }

    before do
      allow(ENV).to receive(:fetch).and_call_original
      allow(ENV).to receive(:fetch).with('API_APP_TOKEN_MAX_AGE_MONTHS', '12').and_return('12')
    end

    it 'defaults to twelve calendar months, including leap-day issuance' do
      token = issue_app_token_at_issued_time

      expect(token.expires_at).to eq(Time.zone.parse('2025-02-28 10:00:00'))
    end

    it 'rejects API and membership authentication at the expiry boundary' do
      token, raw = described_class.issue_for(account: account, household_membership: membership, name: 'Boundary')

      travel_to(token.expires_at, with_usec: true) do
        expect(described_class.lookup_by_token(raw)).to be_nil
        expect(token).not_to be_active_for_membership
      end
    end

    it 'does not extend expiry when used' do
      token = issue_app_token_at_issued_time
      deadline = token.expires_at
      travel_to(issued_at + 1.day) { token.touch_last_used! }

      expect(token.reload.expires_at).to eq(deadline)
    end

    it 'allows a shorter requested expiry' do
      deadline = 1.month.from_now.change(usec: 0)
      token, = described_class.issue_for(account: account, household_membership: membership,
                                         name: 'Shorter', expires_at: deadline)

      expect(token.expires_at).to eq(deadline)
    end

    it 'rejects a requested expiry beyond the configured maximum' do
      expect do
        described_class.issue_for(account: account, household_membership: membership,
                                  name: 'Too long', expires_at: 13.months.from_now)
      end.to raise_error(ActiveRecord::RecordInvalid)
    end

    it 'persists a reduced maximum and never revives the token after an increase' do
      token, raw = described_class.issue_for(account: account, household_membership: membership, name: 'Reduced')
      original_created_at = token.created_at
      allow(ENV).to receive(:fetch).with('API_APP_TOKEN_MAX_AGE_MONTHS', '12').and_return('1')
      travel_to(original_created_at + 2.months) do
        expect(described_class.lookup_by_token(raw)).to be_nil
        expect(token.reload.expires_at).to eq(original_created_at + 1.month)
        allow(ENV).to receive(:fetch).with('API_APP_TOKEN_MAX_AGE_MONTHS', '12').and_return('12')
        expect(described_class.lookup_by_token(raw)).to be_nil
      end
    end

    it 'rejects zero, negative and invalid configured maximum ages' do
      %w[0 -1 invalid].each do |value|
        allow(ENV).to receive(:fetch).with('API_APP_TOKEN_MAX_AGE_MONTHS', '12').and_return(value)
        expect { issue_app_token_at_issued_time }.to raise_error(ArgumentError)
      end
    end

    it 'caps unused tokens at startup without extending already shorter expiries' do
      token, raw = described_class.issue_for(account: account, household_membership: membership, name: 'Unused')
      shorter, = described_class.issue_for(account: account, household_membership: membership,
                                           name: 'Short', expires_at: 1.week.from_now)
      short_deadline = shorter.expires_at
      allow(ENV).to receive(:fetch).with('API_APP_TOKEN_MAX_AGE_MONTHS', '12').and_return('1')

      described_class.apply_maximum_age!

      expect(token.reload.expires_at).to eq(token.created_at + 1.month)
      expect(shorter.reload.expires_at).to eq(short_deadline)
      allow(ENV).to receive(:fetch).with('API_APP_TOKEN_MAX_AGE_MONTHS', '12').and_return('12')
      travel_to(token.created_at + 2.months) { expect(described_class.lookup_by_token(raw)).to be_nil }
    end
  end

  describe '#touch_last_used!' do
    let(:account) { accounts(:jane_doe) }
    let(:membership) { api_membership_for(account) }
    let(:issued_at) { Time.zone.parse('2026-04-21 10:00:00') }

    it 'skips the write when the token was used recently' do
      app_token = issue_app_token_at_issued_time

      travel_to(issued_at + 1.minute) do
        expect do
          app_token.touch_last_used!
        end.not_to(change { app_token.reload.updated_at })
      end
    end

    it 'updates the timestamp when the token has not been used recently' do
      app_token = issue_app_token_at_issued_time

      travel_to(issued_at + 10.minutes) do
        expect do
          app_token.touch_last_used!
        end.to change { app_token.reload.last_used_at }.from(issued_at).to(Time.current)
      end
    end
  end

  describe '#active_for_membership?' do
    let(:account) { accounts(:jane_doe) }
    let(:membership) { api_membership_for(account) }

    it 'requires an attached active membership with the current permissions version' do
      app_token = described_class.issue_for(
        account: account,
        household_membership: membership,
        name: 'RSpec token'
      ).first

      expect(app_token).to be_active_for_membership

      app_token.update!(permissions_version: membership.permissions_version + 1)
      expect(app_token).not_to be_active_for_membership

      app_token.update!(permissions_version: membership.permissions_version)
      create_backup_owner
      membership.update!(status: :revoked)
      expect(app_token).not_to be_active_for_membership
    end

    it 'rejects tokens with no membership' do
      app_token = described_class.new(household_membership: nil)

      expect(app_token).not_to be_active_for_membership
    end

    it 'rejects pre-hosted tokens without a permissions version' do
      app_token = described_class.issue_for(
        account: account,
        household_membership: membership,
        name: 'Legacy token'
      ).first
      app_token.permissions_version = nil

      expect(app_token).not_to be_active_for_membership
    end

    it 'rejects tokens for every non-operational household lifecycle state' do
      app_token = described_class.issue_for(
        account: account,
        household_membership: membership,
        name: 'Hosted lifecycle token'
      ).first

      %i[held offboarded purging purged].each do |lifecycle_state|
        membership.household.update!(lifecycle_state: lifecycle_state)

        expect(app_token.reload).not_to be_active_for_membership
      end
    end
  end

  describe 'validations' do
    let(:account) { accounts(:jane_doe) }
    let(:household) { api_household }

    it 'rejects household memberships from a different account' do
      other_account = Account.create!(email: 'api-token-other-account@example.test', status: :verified)
      token = described_class.new(
        account: account,
        household_membership: mismatched_membership(other_account),
        name: 'Mismatched token',
        token_digest: described_class.digest('mismatched-token'),
        last_used_at: Time.current
      )

      expect(token).not_to be_valid
      expect(token.errors[:household_membership]).to include('must belong to the account')
    end
  end

  def issue_app_token_at_issued_time
    travel_to(issued_at) do
      described_class.issue_for(
        account: account,
        household_membership: membership,
        name: 'RSpec token'
      ).first
    end
  end

  def api_membership_for(account)
    household = api_household
    household.household_memberships.create!(
      account: account,
      person: api_person_for(account, household),
      role: :owner,
      status: :active
    )
  end

  def api_household
    Household.create!(name: "API App Token Spec #{SecureRandom.hex(4)}",
                      slug: "api-app-token-spec-#{SecureRandom.hex(4)}")
  end

  def api_person_for(account, household)
    Person.create!(
      household: household,
      account: account,
      name: 'API App Token Person',
      date_of_birth: 30.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )
  end

  def mismatched_membership(account)
    household.household_memberships.create!(
      account: account,
      person: api_person_for(account, household),
      role: :member,
      status: :active
    )
  end

  def create_backup_owner
    backup_account = Account.create!(email: "api-backup-owner-#{SecureRandom.hex(4)}@example.test", status: :verified)
    membership.household.household_memberships.create!(
      account: backup_account,
      person: api_person_for(backup_account, membership.household),
      role: :owner,
      status: :active
    )
  end
end
