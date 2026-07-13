# frozen_string_literal: true

module Households
  class AccessChange
    class Membership
      ACCESS_FIELDS = %w[role status person_id].freeze

      def initialize(actor_account:, actor_membership:, request:, membership:, attributes:)
        @actor_account = actor_account
        @actor_membership = actor_membership
        @request = request
        @membership = membership
        @attributes = attributes
      end

      def call
        @previous_state = nil
        ActiveRecord::Base.transaction { apply_change! }
        Result.new(true, membership, outcome)
      rescue Rejected => e
        record_event!(e.record, state(e.record), 'rejected')
        Result.new(false, e.record, 'rejected')
      end

      private

      attr_reader :actor_account, :actor_membership, :request, :membership, :attributes, :outcome, :previous_state

      def apply_change!
        lock_records!
        @previous_state = state(membership)
        membership.assign_attributes(attributes)
        reject_invalid_actor!
        reject_unauthorized_owner_change!
        @outcome = changed? ? 'success' : 'no_change'
        persist_change! if outcome == 'success'
        record_event!(membership, state(membership), outcome)
      end

      def lock_records!
        membership.household.lock!
        membership.lock!
      end

      def persist_change!
        membership.save!
        membership.update!(permissions_version: membership.permissions_version + 1)
      rescue ActiveRecord::RecordInvalid
        raise Rejected, membership
      end

      def reject_invalid_actor!
        return if actor_membership&.household_id == membership.household_id
        return if actor_membership.nil? && platform_actor?

        membership.errors.add(:base, 'Access change actor must belong to the household')
        raise Rejected, membership
      end

      def reject_unauthorized_owner_change!
        return unless owner_access_changed?
        return if authorized_owner_change?

        membership.errors.add(:base, owner_change_error)
        raise Rejected, membership
      end

      def authorized_owner_change?
        return platform_actor? if active_owner?
        return actor_owner? || platform_actor? if previous_active_owner?

        actor_owner?
      end

      def previous_active_owner? = previous_state.fetch('role') == 'owner' && previous_state.fetch('status') == 'active'

      def owner_change_error
        return 'Owner promotion requires an active platform administrator' if active_owner?

        'Only household owners can change owner memberships'
      end

      def active_owner?
        membership.owner? && membership.active?
      end

      def owner_access_changed?
        previous_owner = previous_state.fetch('role') == 'owner' && previous_state.fetch('status') == 'active'
        previous_owner != active_owner?
      end

      def platform_actor?
        actor_account&.platform_admin&.active?
      end

      def actor_owner?
        actor_membership && HouseholdMembership.owner.active.exists?(id: actor_membership.id)
      end

      def changed?
        ACCESS_FIELDS.any? { |field| membership.will_save_change_to_attribute?(field) }
      end

      def state(record)
        {
          'role' => record.role,
          'status' => record.status,
          'person_id' => record.person_id
        }
      end

      def record_event!(record, new_state, event_outcome)
        Audit::Event.record!(
          household: record.household,
          actor_account: actor_account,
          actor_membership: actor_membership,
          event_type: event_type(new_state),
          request: request,
          metadata: event_metadata(record, new_state, event_outcome)
        )
      end

      def event_type(new_state)
        return 'household_membership.role_updated' if previous_state.fetch('role') != new_state.fetch('role')

        'household_access.membership_changed'
      end

      def event_metadata(record, new_state, event_outcome)
        {
          target_account_id: record.account_id,
          target_membership_id: record.id,
          previous_role: previous_state.fetch('role'),
          new_role: new_state.fetch('role'),
          previous_state: previous_state,
          new_state: new_state,
          outcome: event_outcome
        }
      end
    end
  end
end
