# frozen_string_literal: true

module Platform
  class HouseholdMembershipsController < BaseController
    def promote_owner
      household = Household.find(params.expect(:household_id))
      membership = HouseholdMembership.find(params.expect(:id))
      authorize membership, :promote_owner?, policy_class: PlatformHouseholdMembershipPolicy
      return reject_household_mismatch(household, membership) if membership.household_id != household.id

      result = access_change.promote_owner(membership)
      if result.success?
        redirect_to platform_users_path, notice: t('platform.users.owner_promoted')
      else
        redirect_to platform_users_path,
                    alert: t('platform.users.owner_promotion_rejected'),
                    status: :see_other
      end
    end

    private

    def require_platform_admin
      allowed = current_account&.platform_admin&.active?
      super
      record_promotion_rejection('platform_administrator_required') unless allowed
    end

    def require_privileged_action_mfa
      super
      record_promotion_rejection('fresh_privileged_action_required') if performed?
    end

    def access_change
      @access_change ||= Households::AccessChange.new(
        actor_account: current_account,
        actor_membership: nil,
        request: request
      )
    end

    def reject_household_mismatch(household, membership)
      record_promotion_rejection('target_household_mismatch', household: household, membership: membership)
      head :not_found
    end

    def record_promotion_rejection(reason, household: promotion_household, membership: promotion_membership)
      Audit::Event.record!(
        household: household,
        actor_account: current_account,
        event_type: 'household_owner_promotion.rejected',
        request: request,
        metadata: {
          target_membership_id: membership&.id || params[:id]&.to_i,
          previous_role: membership&.role,
          new_role: 'owner',
          outcome: 'rejected',
          reason: reason
        }
      )
    end

    def promotion_household
      Household.find_by(id: params[:household_id])
    end

    def promotion_membership
      HouseholdMembership.find_by(id: params[:id])
    end
  end
end
