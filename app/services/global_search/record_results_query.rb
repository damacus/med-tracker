# frozen_string_literal: true

module GlobalSearch
  class RecordResultsQuery
    include Rails.application.routes.url_helpers

    attr_reader :user, :query, :limit, :builder

    def initialize(user:, query:, limit:, builder:)
      @user = user
      @query = query
      @limit = limit
      @builder = builder
    end

    private

    def default_url_options
      household = policy_household
      return {} unless household

      { household_slug: household.slug }
    end

    def tenant_route_args(record)
      [household_slug_for(record), record]
    end

    def tenant_route_options(record, **options)
      options.merge(household_slug: household_slug_for(record))
    end

    def household_slug_for(record)
      current_household_slug || record_household_slug(record) || user_household_slug
    end

    def current_household_slug
      Current.household&.slug
    end

    def record_household_slug(record)
      record_household(record)&.slug
    end

    def user_household_slug
      household = user.person&.household || user_account&.first_active_household
      household&.slug
    end

    def user_account
      user.person&.account
    end

    def policy_user
      AuthorizationContext.current || derived_authorization_context || user
    end

    def derived_authorization_context
      membership = user_account&.first_active_household_membership
      return unless membership

      AuthorizationContext.new(account: user_account, household: membership.household, membership: membership)
    end

    def policy_household
      household_candidates.compact.first
    end

    def household_candidates
      [
        Current.household,
        AuthorizationContext.current&.household,
        derived_authorization_context&.household,
        user.person&.household,
        user_account&.first_active_household
      ]
    end

    def record_household(record)
      return record.household if record.respond_to?(:household)
      return record.person.household if record.respond_to?(:person)

      nil
    end

    def scoped(model)
      Pundit.policy_scope!(policy_user, model)
    end

    def search_term
      @search_term ||= "%#{ActiveRecord::Base.sanitize_sql_like(query)}%"
    end
  end
end
