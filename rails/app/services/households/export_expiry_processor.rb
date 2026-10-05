# frozen_string_literal: true

module Households
  class ExportExpiryProcessor
    class << self
      def call
        Household.find_each.sum { |household| expire_household(household) }
      end

      private

      def expire_household(household)
        TenantContext.with(account: nil, household: household) do
          household.with_lock do
            next 0 if HouseholdRetentionHold.active.exists?(household: household)

            due_exports(household).count { |export| HostedExport.expire_due!(export: export) }
          end
        end
      end

      def due_exports(household)
        household_exports = HouseholdExport.where(household: household)
        retention_expired = household_exports.where(status: %i[ready downloaded], expires_at: ..Time.current)
        generation_stale = household_exports.where(
          status: :generating,
          generation_started_at: ..HostedExport.generation_timeout.ago
        )
        retention_expired.or(generation_stale).find_each
      end
    end
  end
end
