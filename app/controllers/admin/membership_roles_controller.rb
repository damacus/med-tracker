# frozen_string_literal: true

module Admin
  class MembershipRolesController < BaseController
    def update
      @user = policy_scope(User).find(params.expect(:id))
      authorize @user, :update?

      result = update_membership_role
      if result.success?
        redirect_to admin_users_path, notice: result.message
      else
        redirect_to admin_users_path, alert: result.message
      end
    end

    private

    def update_membership_role
      Admin::MembershipRoleUpdater.new(
        membership: membership,
        role: membership_params[:role],
        actor_account: current_account,
        actor_membership: admin_target_membership,
        request: request
      ).call
    end

    def membership
      admin_target_household.household_memberships.active.find_by!(account: @user.person.account)
    end

    def membership_params
      params.expect(membership: [:role])
    end

    def admin_target_household
      current_household || default_household_for_urls
    end

    def admin_target_membership
      current_membership || current_account&.active_household_membership_for(admin_target_household)
    end
  end
end
