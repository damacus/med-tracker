# frozen_string_literal: true

module Admin
  class AmbiguousPersonAccessGrantsController < BaseController
    def index
      authorize PersonAccessGrant, :index?
      grants = Admin::AmbiguousPersonAccessGrantsIndexQuery.new(scope: policy_scope(PersonAccessGrant)).call
      @pagy, grants = pagy(:offset, grants)

      render Components::Admin::AmbiguousPersonAccessGrants::IndexView.new(
        grants: grants,
        pagy: @pagy
      )
    end
  end
end
