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
      failure = validation_failure
      return failure if failure

      result = access_change.update_membership(membership, role: role)
      return Result.new(true, I18n.t('admin.membership_roles.updated')) if result.success?

      Result.new(false, access_change_error_message)
    end

    private

    attr_reader :membership, :role, :actor_account, :actor_membership, :request

    def allowed_role?
      ALLOWED_ROLES.include?(role)
    end

    def validation_failure
      return if allowed_role? || role == OWNER_ROLE

      Result.new(false, I18n.t('admin.membership_roles.invalid_role'))
    end

    def access_change
      @access_change ||= Households::AccessChange.new(
        actor_account: actor_account,
        actor_membership: actor_membership,
        request: request
      )
    end

    def access_change_error_message
      if role == OWNER_ROLE
        I18n.t('admin.membership_roles.owner_rejected')
      elsif membership.role_before_last_save == OWNER_ROLE || membership.role_in_database == OWNER_ROLE
        I18n.t('admin.membership_roles.owner_demotion_rejected')
      else
        membership.errors.full_messages.to_sentence
      end
    end
  end
end
