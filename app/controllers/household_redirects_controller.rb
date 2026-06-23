# frozen_string_literal: true

class HouseholdRedirectsController < ApplicationController
  def show
    membership = current_account&.first_active_household_membership

    return user_not_authorized unless membership

    redirect_to dashboard_path(household_slug: membership.household.slug), status: :see_other
  end
end
