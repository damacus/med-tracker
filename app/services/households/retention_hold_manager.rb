# frozen_string_literal: true

module Households
  class RetentionHoldManager
    extend OperatorAuthorization

    class UnavailableHousehold < StandardError
      def initialize
        super('Household purge has already started')
      end
    end

    class << self
      def place!(household:, actor_account:, reason:, review_on:)
        authorize_operator!(actor_account)
        TenantContext.with(account: actor_account, household: household) do
          ActiveRecord::Base.transaction do
            place_locked!(household, actor_account, reason, review_on)
          end
        end
      end

      def release!(hold:, actor_account:)
        authorize_operator!(actor_account)
        TenantContext.with(account: actor_account, household: hold.household) do
          ActiveRecord::Base.transaction do
            hold.lock!
            return hold if hold.released?

            hold.update!(status: :released, released_by_account: actor_account, released_at: Time.current)
            hold.household.update!(lifecycle_state: :active) if hold.household.held?
            record_event(hold, actor_account, 'released')
            hold
          end
        end
      end

      private

      def place_locked!(household, actor_account, reason, review_on)
        household.lock!
        raise UnavailableHousehold if household.purging? || household.purged?

        hold = household.household_retention_holds.create!(
          approved_by_account: actor_account,
          reason: reason,
          review_on: review_on,
          placed_at: Time.current
        )
        household.update!(lifecycle_state: :held) if household.operational?
        record_event(hold, actor_account, 'placed')
        hold
      end

      def record_event(hold, actor_account, action)
        Audit::Event.record!(
          household: hold.household,
          actor_account: actor_account,
          event_type: "household.retention_hold.#{action}",
          metadata: {
            retention_hold_id: hold.id,
            approver_account_id: hold.approved_by_account_id,
            review_on: hold.review_on.iso8601,
            outcome: action
          }
        )
      end
    end
  end
end
