# frozen_string_literal: true

module Households
  class AccessChange
    class PersonGrant
      ACCESS_FIELDS = %w[
        household_membership_id person_id access_level relationship_type expires_at revoked_at
      ].freeze

      def initialize(actor_account:, actor_membership:, request:, grant:, attributes:)
        @actor_account = actor_account
        @actor_membership = actor_membership
        @request = request
        @grant = grant
        @attributes = attributes
      end

      def call
        prepare_change
        ActiveRecord::Base.transaction { apply_change! }
        Result.new(true, grant, outcome)
      rescue Rejected => e
        record_event!(e.record, rejected_household(e.record), state(e.record), 'rejected')
        Result.new(false, e.record, 'rejected')
      end

      private

      attr_reader :actor_account, :actor_membership, :request, :grant, :attributes, :household,
                  :previous_state, :outcome

      def prepare_change
        @previous_state = nil
        @household = find_household
      end

      def apply_change!
        reject_missing_household! unless household
        lock_change!
        assign_change!
        reject_invalid_actor!
        reject_cross_household!
        @outcome = changed? ? 'success' : 'no_change'
        persist_change! if outcome == 'success'
        record_event!(grant, household, state(grant), outcome)
      end

      def lock_change!
        household.lock!
        grant.lock! if grant.persisted?
        @previous_state = grant.persisted? ? state(grant) : nil
        @previous_membership = grant.household_membership
      end

      def assign_change!
        grant.assign_attributes(attributes)
        @affected_memberships = [@previous_membership, grant.household_membership].compact.uniq.sort_by(&:id)
        @affected_memberships.each(&:lock!)
      end

      def persist_change!
        grant.save!
        @affected_memberships.each do |affected_membership|
          affected_membership.update!(permissions_version: affected_membership.permissions_version + 1)
        end
      rescue ActiveRecord::RecordInvalid
        raise Rejected, grant
      end

      def reject_missing_household!
        grant.errors.add(:household, 'must exist')
        raise Rejected, grant
      end

      def reject_invalid_actor!
        return if actor_membership&.household_id == household.id
        return if actor_membership.nil? && actor_account&.platform_admin&.active?

        grant.errors.add(:base, 'Access change actor must belong to the household')
        raise Rejected, grant
      end

      def reject_cross_household!
        return if household_records.all? { |record| record.nil? || record.household_id == household.id }

        grant.errors.add(:base, 'Person access grant records must belong to the household')
        raise Rejected, grant
      end

      def household_records
        [grant, grant.household_membership, grant.person, grant.granted_by_membership]
      end

      def changed?
        grant.new_record? || ACCESS_FIELDS.any? { |field| grant.will_save_change_to_attribute?(field) }
      end

      def find_household
        return grant.household if grant.persisted?

        supplied_household || supplied_membership&.household
      end

      def supplied_household
        attributes[:household] || attributes['household'] ||
          Household.find_by(id: attributes[:household_id] || attributes['household_id'])
      end

      def supplied_membership
        attributes[:household_membership] || attributes['household_membership'] ||
          HouseholdMembership.find_by(id: attributes[:household_membership_id] || attributes['household_membership_id'])
      end

      def rejected_household(record)
        record.household || record.household_membership&.household
      end

      def state(record)
        {
          'household_membership_id' => record.household_membership_id,
          'person_id' => record.person_id,
          'access_level' => record.access_level,
          'relationship_type' => record.relationship_type,
          'expires_at' => record.expires_at&.iso8601,
          'revoked_at' => record.revoked_at&.iso8601
        }
      end

      def record_event!(record, event_household, new_state, event_outcome)
        Audit::Event.record!(
          household: event_household,
          actor_account: actor_account,
          actor_membership: actor_membership,
          event_type: 'household_access.person_grant_changed',
          request: request,
          metadata: {
            target_membership_id: record.household_membership_id,
            target_grant_id: record.id,
            previous_state: previous_state,
            new_state: new_state,
            outcome: event_outcome
          }
        )
      end
    end
  end
end
