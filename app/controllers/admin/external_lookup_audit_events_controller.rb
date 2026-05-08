# frozen_string_literal: true

module Admin
  class ExternalLookupAuditEventsController < ApplicationController
    EVENTS_PER_PAGE = 50

    before_action :authorize_access

    def index
      page = [params[:page].to_i, 1].max
      filtered = filtered_scope

      @total_count = filtered.count
      @events = filtered.order(created_at: :desc)
                        .limit(EVENTS_PER_PAGE)
                        .offset((page - 1) * EVENTS_PER_PAGE)

      render Components::Admin::ExternalLookupAuditEvents::IndexView.new(
        events: @events,
        filter_params: params.permit(:source, :result_status),
        current_page: page,
        total_count: @total_count,
        per_page: EVENTS_PER_PAGE
      )
    end

    private

    def authorize_access
      authorize :audit_log, :index?, policy_class: AuditLogPolicy
    end

    def filtered_scope
      scope = ExternalLookupAuditEvent.all
      scope = scope.where(source: params[:source]) if params[:source].present?
      scope = scope.where(result_status: params[:result_status]) if params[:result_status].present?
      scope
    end
  end
end
