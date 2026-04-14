# frozen_string_literal: true

module Admin
  class AuditLogsQuery
    Result = Data.define(:versions, :total_count, :page, :per_page)

    attr_reader :scope, :filters, :page, :per_page

    def initialize(scope:, filters:, page:, per_page:)
      @scope = scope
      @filters = filters
      @page = [page.to_i, 1].max
      @per_page = per_page
    end

    def call
      Result.new(
        versions: filtered_scope.limit(per_page).offset((page - 1) * per_page),
        total_count: filtered_scope.count,
        page: page,
        per_page: per_page
      )
    end

    private

    def filtered_scope
      relation = scope.order(created_at: :desc)
      relation = relation.where(item_type: item_type) if item_type.present?
      relation = relation.where(event: event) if event.present?
      relation = relation.where(whodunnit: whodunnit) if whodunnit.present?
      relation
    end

    def item_type
      filters[:item_type].to_s.presence
    end

    def event
      filters[:event].to_s.presence
    end

    def whodunnit
      filters[:whodunnit].to_s.presence
    end
  end
end
