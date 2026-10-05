# frozen_string_literal: true

class HouseholdRedirectsController < ApplicationController
  skip_after_action :verify_pundit_authorization

  def show
    membership = first_active_household_membership

    return redirect_to_login_without_household unless membership

    redirect_to dashboard_path(household_slug: membership.household.slug), status: :see_other
  end

  private

  def first_active_household_membership
    return unless current_account

    TenantContext.with(account: current_account, household: nil, request_id: request.request_id) do
      current_account.first_active_household_membership
    end
  end

  def redirect_to_login_without_household
    reset_session
    redirect_to rodauth.login_path,
                alert: t('pundit.not_authorized', default: 'You are not authorized to perform this action.'),
                status: :see_other
  end
end
