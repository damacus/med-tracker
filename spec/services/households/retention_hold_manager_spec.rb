# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Households::RetentionHoldManager do
  def account(email)
    Account.create!(email: email, status: :verified)
  end

  def owner_membership(household, owner)
    household.household_memberships.create!(
      account: owner,
      role: :owner,
      status: :active,
      joined_at: Time.current
    )
  end

  let(:household) { create(:household) }
  let(:operator) { account('hosted-lifecycle-operator@example.test') }

  before { PlatformAdmin.create!(account: operator) }

  describe Households::RetentionHoldManager do
    it 'places and releases an audited hold while changing household availability', :aggregate_failures do
      hold = place_hold('Regulatory preservation request')
      expect(hold).to be_active
      verify_placed_hold(hold)
      described_class.release!(hold: hold, actor_account: operator)
      verify_released_hold(hold)
    end

    def place_hold(reason)
      described_class.place!(household: household, actor_account: operator, reason: reason,
                             review_on: 30.days.from_now.to_date)
    end

    def verify_placed_hold(hold)
      expect([hold.approved_by_account, household.reload.lifecycle_state, household.operational?])
        .to eq([operator, 'held', false])
      expect(audit_events('household.retention_hold.placed')).to exist
    end

    def verify_released_hold(hold)
      expect([hold.reload.status, hold.released_by_account, household.reload.operational?])
        .to eq(['released', operator, true])
      expect(audit_events('household.retention_hold.released')).to exist
    end

    def audit_events(event_type) = SecurityAuditEvent.where(household: household, event_type: event_type)

    it 'requires a reason and future review date from a platform administrator' do
      expect do
        described_class.place!(
          household: household,
          actor_account: account('ordinary-retention-actor@example.test'),
          reason: '',
          review_on: Date.current
        )
      end.to raise_error(Pundit::NotAuthorizedError)
    end

    it 'preserves an offboarded household state while a hold is placed and released' do
      Households::Offboarder.call(household: household, actor_account: operator)

      hold = described_class.place!(
        household: household,
        actor_account: operator,
        reason: 'Post-offboarding preservation request',
        review_on: 30.days.from_now.to_date
      )
      described_class.release!(hold: hold, actor_account: operator)

      expect(household.reload).to be_offboarded
      expect(household).not_to be_operational
    end

    it 'refuses a new hold after purge has started' do
      household.update!(status: :archived, lifecycle_state: :purging)

      expect do
        described_class.place!(
          household: household,
          actor_account: operator,
          reason: 'Late preservation request',
          review_on: 30.days.from_now.to_date
        )
      end.to raise_error(described_class::UnavailableHousehold)
    end
  end

  describe Households::Offboarder do
    it 'atomically disables access and revokes every hosted credential and device surface', :aggregate_failures do
      records = offboarding_records
      other_records = preserved_household_records
      create_second_browser_session
      counts = offboard_twice
      expect(household.reload).to be_offboarded
      verify_revoked_records(records)
      verify_offboard_idempotency(counts)
      verify_preserved_records(other_records)
    end

    def offboarding_records
      member_account = account('hosted-offboard-member@example.test')
      membership = owner_membership(household, member_account)
      credentials = issue_credentials(member_account, membership)
      credentials.merge(create_devices(member_account))
                 .merge(member_account: member_account, membership: membership)
    end

    def issue_credentials(member_account, membership)
      api_session = ApiSession.issue_for(account: member_account, household_membership: membership).first
      app_token = ApiAppToken.issue_for(
        account: member_account,
        household_membership: membership,
        name: 'Offboard test token'
      ).first
      { api_session: api_session, app_token: app_token, oauth_grant: create_oauth_grant(member_account, membership) }
    end

    def create_oauth_grant(member_account, membership)
      OauthGrant.create!(
        account: member_account,
        household_membership: membership,
        person: create(:person, household: household, account: member_account),
        oauth_application: OauthApplication.create!(
          name: 'Offboard test app',
          client_id: SecureRandom.hex(16),
          redirect_uri: 'https://example.test/callback',
          scopes: 'patient/*.rs'
        ),
        expires_in: 1.hour.from_now,
        permissions_version: membership.permissions_version,
        scopes: 'patient/*.rs'
      )
    end

    def create_devices(member_account)
      push_subscription = create_push_subscription(member_account)
      device_token = create_native_device_token(member_account)
      create_browser_session(member_account)
      { push_subscription: push_subscription, device_token: device_token }
    end

    def create_push_subscription(member_account)
      PushSubscription.create!(
        account: member_account,
        endpoint: 'https://fcm.googleapis.com/fcm/send/offboard-test',
        p256dh: 'p256dh',
        auth: 'auth'
      )
    end

    def create_native_device_token(member_account)
      NativeDeviceToken.create!(
        account: member_account,
        device_token: SecureRandom.hex(32),
        platform: 'ios'
      )
    end

    def create_browser_session(member_account)
      ActiveRecord::Base.connection.execute(<<~SQL.squish)
        INSERT INTO account_active_session_keys (account_id, session_id)
        VALUES (#{member_account.id}, 'offboard-browser-session')
      SQL
    end

    def preserved_household_records
      other_household = create(:household)
      { household: other_household, location: create(:location, household: other_household) }
    end

    def create_second_browser_session
      second_member = account('hosted-offboard-second-member@example.test')
      household.household_memberships.create!(account: second_member, role: :member, status: :active)
      AccountActiveSessionKey.create!(account: second_member, session_id: 'second-offboard-browser-session')
    end

    def offboard_twice
      described_class.call(household: household, actor_account: operator)
      counts = {
        audits: SecurityAuditEvent.where(household:, event_type: 'household.offboarded').count,
        versions: PaperTrail::Version.where(household_id: household.id).count
      }
      described_class.call(household: household, actor_account: operator)
      counts
    end

    def verify_revoked_records(records)
      expect(records.fetch(:membership).reload).to be_revoked
      expect(records.values_at(:api_session, :app_token, :oauth_grant)).to all(satisfy { |record| revoked?(record) })
      verify_destroyed_devices(records)
    end

    def revoked?(record) = record.reload.revoked_at.present?

    def verify_destroyed_devices(records)
      push_exists = PushSubscription.exists?(id: records.fetch(:push_subscription).id)
      token_exists = NativeDeviceToken.exists?(id: records.fetch(:device_token).id)
      session_keys = AccountActiveSessionKey.where(account_id: household.household_memberships.select(:account_id))
      expect([push_exists, token_exists, session_keys.exists?]).to all(be(false))
    end

    def verify_offboard_idempotency(counts)
      current = [
        SecurityAuditEvent.where(household:, event_type: 'household.offboarded').count,
        PaperTrail::Version.where(household_id: household.id).count
      ]
      expect([counts.fetch(:audits), current]).to eq([1, [counts.fetch(:audits), counts.fetch(:versions)]])
    end

    def verify_preserved_records(records)
      expect(records.fetch(:household).reload).to be_operational
      expect(records.fetch(:location).reload).to be_present
    end

    it 'offboards idempotently when the household has no member accounts' do
      empty_household = create(:household)

      described_class.call(household: empty_household, actor_account: operator)
      described_class.call(household: empty_household, actor_account: operator)

      expect(empty_household.reload).to be_offboarded
      expect(SecurityAuditEvent.where(household: empty_household, event_type: 'household.offboarded').count).to eq(1)
    end

    it 'preserves account-scoped credentials when an account retains another operational household', :aggregate_failures do
      shared_account = account('shared-offboard-member@example.test')
      target_membership = owner_membership(household, shared_account)
      other_household = create(:household)
      other_membership = owner_membership(other_household, shared_account)
      api_session = ApiSession.issue_for(account: shared_account, household_membership: target_membership).first
      devices = create_devices(shared_account)

      described_class.call(household: household, actor_account: operator)

      expect(api_session.reload.revoked_at).to be_present
      expect(other_membership.reload).to be_active
      expect(PushSubscription.where(id: devices.fetch(:push_subscription).id)).to exist
      expect(NativeDeviceToken.where(id: devices.fetch(:device_token).id)).to exist
      expect(AccountActiveSessionKey.where(account_id: shared_account.id)).to exist
    end
  end
end
