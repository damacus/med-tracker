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
          household.lock!
          next 0 if HouseholdRetentionHold.active.exists?(household: household)

          due_exports(household).count { |export| HostedExport.expire_due!(export: export) }
        end
      end

      def due_exports(household)
        HouseholdExport.where(household: household, status: %i[ready downloaded], expires_at: ..Time.current).find_each
      end
    end
  end
end
