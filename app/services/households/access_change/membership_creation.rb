# frozen_string_literal: true

module Households
  class AccessChange
    class MembershipCreation
      INITIAL_PERMISSIONS_VERSION = 1

      def initialize(actor_account:, actor_membership:, request:, household:, attributes:)
        @actor_account = actor_account
        @actor_membership = actor_membership
        @request = request
        @household = household
        @membership = HouseholdMembership.new(household: household)
        @attributes = attributes.except(:permissions_version, 'permissions_version')
      end

      def call
        ActiveRecord::Base.transaction { apply_change! }
        Result.new(true, membership, 'success')
      rescue Rejected => e
        record_event!(e.record, 'rejected')
        Result.new(false, e.record, 'rejected')
      end

      private

      attr_reader :actor_account, :actor_membership, :request, :household, :membership, :attributes

      def apply_change!
        reject_missing_household! unless household&.persisted?
        household.lock!
        membership.assign_attributes(attributes)
        membership.permissions_version = INITIAL_PERMISSIONS_VERSION
        reject_invalid_actor!
        reject_cross_household_person!
        reject_ungoverned_owner!
        persist_membership!
        record_event!(membership, 'success')
      end

      def persist_membership!
        membership.save!
      rescue ActiveRecord::RecordInvalid
        raise Rejected, membership
      end

      def reject_missing_household!
        membership.errors.add(:household, 'must exist')
        raise Rejected, membership
      end

      def reject_invalid_actor!
        return if active_household_actor?
        return if platform_actor?
        return if owner_bootstrap?

        membership.errors.add(:base, 'Access change actor must belong to the household')
        raise Rejected, membership
      end

      def reject_cross_household_person!
        return if membership.person.nil? || membership.person.household_id == household.id

        membership.errors.add(:person, 'must belong to the same household')
        raise Rejected, membership
      end

      def reject_ungoverned_owner!
        return unless membership.owner?
        return if owner_bootstrap?

        membership.errors.add(:base, 'Owner memberships must use the governed promotion path')
        raise Rejected, membership
      end

      def active_household_actor?
        actor_membership&.household_id == household.id &&
          HouseholdMembership.active.exists?(id: actor_membership.id)
      end

      def platform_actor?
        actor_membership.nil? && actor_account&.platform_admin&.active?
      end

      def owner_bootstrap?
        membership.owner? && membership.active? && first_membership? && bootstrap_identity_matches?
      end

      def first_membership?
        household.household_memberships.none?
      end

      def bootstrap_identity_matches?
        household.created_by_account_id == membership.account_id &&
          actor_account == membership.account && actor_membership.nil?
      end

      def record_event!(record, event_outcome)
        Audit::Event.record!(
          household: household,
          actor_account: actor_account,
          actor_membership: actor_membership,
          event_type: 'household_access.membership_created',
          request: request,
          metadata: {
            target_account_id: record.account_id,
            target_membership_id: record.id,
            previous_state: nil,
            new_state: state(record),
            outcome: event_outcome
          }
        )
      end

      def state(record)
        {
          'role' => record.role,
          'status' => record.status,
          'person_id' => record.person_id,
          'permissions_version' => record.permissions_version
        }
      end
    end
  end
end
