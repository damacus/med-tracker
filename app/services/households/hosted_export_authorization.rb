# frozen_string_literal: true

module Households
  module HostedExportAuthorization
    private

    def with_locked_household(export, actor_account, &)
      household = export.household
      TenantContext.with(account: actor_account, household: household) do
        household.with_lock(&)
      end
    end

    def authorize_membership!(household, membership, actor_account)
      manager = membership&.household_id == household.id && membership.active? &&
                (membership.owner? || membership.administrator?)
      return if manager && actor_authorized?(household, membership, actor_account)

      raise Pundit::NotAuthorizedError, 'Active household manager access is required'
    end

    def actor_authorized?(household, membership, actor_account)
      return true if membership.account_id == actor_account&.id

      platform_admin = actor_account&.platform_admin
      platform_admin&.active? && platform_admin.support_access_sessions.active.exists?(household: household)
    end

    def authorize_export_actor!(export, actor_account)
      authorized = TenantContext.with(account: actor_account, household: export.household) do
        authorized_household_manager?(export.household, actor_account) ||
          authorized_platform_operator?(export.household, actor_account)
      end
      return if authorized

      raise Pundit::NotAuthorizedError, 'Export access is not authorized'
    end

    def authorized_household_manager?(household, actor_account)
      household.operational? &&
        household.household_memberships.active.exists?(account: actor_account, role: %i[owner administrator])
    end

    def authorized_platform_operator?(household, actor_account)
      platform_admin = actor_account&.platform_admin
      platform_admin&.active? && platform_admin.support_access_sessions.active.exists?(household: household)
    end
  end
end
