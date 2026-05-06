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

    def scoped(model)
      Pundit.policy_scope!(user, model)
    end

    def search_term
      @search_term ||= "%#{ActiveRecord::Base.sanitize_sql_like(query)}%"
    end
  end
end
