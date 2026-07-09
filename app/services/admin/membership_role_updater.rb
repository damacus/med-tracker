# frozen_string_literal: true

module Admin
  class MembershipRoleUpdater
    OWNER_ROLE = 'owner'
    ALLOWED_ROLES = %w[administrator member].freeze

    Result = Data.define(:success?, :message)

    def initialize(membership:, role:, actor_account:, actor_membership:, request:)
      @membership = membership
      @role = role.to_s
      @actor_account = actor_account
      @actor_membership = actor_membership
      @request = request
    end

    def call
      return Result.new(false, I18n.t('admin.membership_roles.owner_rejected')) if role == OWNER_ROLE
      return Result.new(false, I18n.t('admin.membership_roles.invalid_role')) unless allowed_role?
      return Result.new(false, I18n.t('admin.membership_roles.owner_demotion_rejected')) unless owner_change_allowed?

      previous_role = membership.role
      ActiveRecord::Base.transaction do
        membership.update!(role: role)
        record_audit_event(previous_role)
      end
      Result.new(true, I18n.t('admin.membership_roles.updated'))
    end

    private

    attr_reader :membership, :role, :actor_account, :actor_membership, :request

    def allowed_role?
      ALLOWED_ROLES.include?(role)
    end

    def owner_change_allowed?
      owner_governance.can_change_owner_membership?(membership)
    end

    def owner_governance
      @owner_governance ||= OwnerGovernance.new(
        household: membership.household,
        actor_membership: actor_membership
      )
    end

    def record_audit_event(previous_role)
      SecurityAuditEvent.create!(
        household: membership.household,
        actor_account: actor_account,
        actor_membership: actor_membership,
        event_type: 'household_membership.role_updated',
        request_id: request.request_id,
        ip: request.remote_ip,
        metadata: {
          target_account_id: membership.account_id,
          target_membership_id: membership.id,
          previous_role: previous_role,
          new_role: membership.role
        }
      )
    end
  end
end
