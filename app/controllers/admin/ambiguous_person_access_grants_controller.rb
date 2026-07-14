# frozen_string_literal: true

module Admin
  class AmbiguousPersonAccessGrantsController < BaseController
    skip_before_action :require_hosted_privileged_action_mfa
    before_action :require_privileged_action_mfa, if: :ambiguous_grants_mfa_required?

    def index
      authorize PersonAccessGrant, :index?
      grants = Admin::AmbiguousPersonAccessGrantsIndexQuery.new(scope: policy_scope(PersonAccessGrant)).call
      @pagy, grants = pagy(:offset, grants)

      render Components::Admin::AmbiguousPersonAccessGrants::IndexView.new(
        grants: grants,
        pagy: @pagy
      )
    end

    private

    def ambiguous_grants_mfa_required?
      current_membership&.active? && (current_membership.owner? || current_membership.administrator?)
    end
  end
end
