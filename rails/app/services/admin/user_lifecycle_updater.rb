# frozen_string_literal: true

module Admin
  class UserLifecycleUpdater
    Result = Data.define(:success?, :message)

    def initialize(user:, action:, actor:, household:, actor_membership:)
      @user = user
      @action = action.to_sym
      @actor = actor
      @household = household
      @actor_membership = actor_membership
    end

    def call
      case action
      when :activate
        activate
      when :deactivate
        deactivate
      when :verify
        verify
      else
        raise ArgumentError, "Unsupported user lifecycle action: #{action}"
      end
    end

    private

    attr_reader :user, :action, :actor, :household, :actor_membership

    def activate
      user.activate!
      success('users.activated')
    end

    def deactivate
      return failure('users.cannot_deactivate_self') if user == actor
      return failure('users.owner_deactivation_rejected') if owner_deactivation_blocked?

      user.deactivate!
      success('users.deactivated')
    end

    def verify
      account = user.person&.account
      return failure('admin.users.missing_account') unless account

      ActiveRecord::Base.transaction do
        account.update!(status: :verified)
        AccountVerificationKey.where(account_id: account.id).delete_all
      end
      success('users.verified')
    end

    def owner_deactivation_blocked?
      !owner_governance.can_deactivate_owner_user?(target_membership)
    end

    def owner_governance
      @owner_governance ||= OwnerGovernance.new(
        household: household,
        actor_membership: actor_membership
      )
    end

    def target_membership
      account = user.person&.account
      return unless household && account

      household.household_memberships.active.find_by(account: account)
    end

    def success(key)
      Result.new(true, I18n.t(key))
    end

    def failure(key)
      Result.new(false, I18n.t(key))
    end
  end
end
