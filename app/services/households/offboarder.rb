# frozen_string_literal: true

module Households
  class Offboarder
    extend OperatorAuthorization

    class << self
      def call(household:, actor_account:)
        authorize_operator!(actor_account)
        TenantContext.with(account: actor_account, household: household) do
          ActiveRecord::Base.transaction do
            household.lock!
            return household if household.offboarded? || household.purging? || household.purged?

            household.update!(status: :archived, lifecycle_state: :offboarded, offboarded_at: Time.current)
            revoke_access!(household, actor_account)
            record_event(household, actor_account)
            household
          end
        end
      end

      private

      def revoke_access!(household, actor_account)
        memberships = household.household_memberships.to_a
        access_change = AccessChange.new(actor_account: actor_account, actor_membership: nil, request: nil)

        revoke_person_grants!(household, access_change)
        revoke_credentials!(memberships)
        end_support_sessions!(household)
        revoke_memberships!(memberships, access_change)
        destroy_device_credentials!(memberships.map(&:account_id))
      end

      def revoke_person_grants!(household, access_change)
        PersonAccessGrant.active.where(household: household).find_each do |grant|
          access_change.revoke_grant!(grant)
        end
      end

      def revoke_credentials!(memberships)
        membership_ids = memberships.map(&:id)
        revoke_records!(ApiSession.where(household_membership_id: membership_ids, revoked_at: nil))
        revoke_records!(ApiAppToken.where(household_membership_id: membership_ids, revoked_at: nil))
        revoke_records!(OauthGrant.where(household_membership_id: membership_ids, revoked_at: nil))
      end

      def revoke_records!(scope)
        scope.find_each { |record| record.update!(revoked_at: Time.current) }
      end

      def end_support_sessions!(household)
        SupportAccessSession.where(household: household, ended_at: nil).find_each do |session|
          session.update!(ended_at: Time.current)
        end
      end

      def revoke_memberships!(memberships, access_change)
        memberships.each do |membership|
          access_change.update_membership!(membership, status: :revoked, revoked_at: Time.current)
        end
      end

      def destroy_device_credentials!(account_ids)
        removable_account_ids = account_ids - operational_account_ids(account_ids)
        PushSubscription.where(account_id: removable_account_ids).destroy_all
        NativeDeviceToken.where(account_id: removable_account_ids).destroy_all
        AccountActiveSessionKey.where(account_id: removable_account_ids).destroy_all
      end

      def operational_account_ids(account_ids)
        HouseholdMembership.active.joins(:household).merge(Household.operational)
                           .where(account_id: account_ids).distinct.pluck(:account_id)
      end

      def record_event(household, actor_account)
        Audit::Event.record!(
          household: household,
          actor_account: actor_account,
          event_type: 'household.offboarded',
          metadata: { household_id: household.id, outcome: 'success' }
        )
      end
    end
  end
end
