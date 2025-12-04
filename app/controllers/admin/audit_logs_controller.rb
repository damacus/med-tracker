# frozen_string_literal: true

module Admin
  # Controller for viewing audit trail logs
  # Provides read-only access to PaperTrail versions for administrators
  # @see docs/audit-trail.md
  class AuditLogsController < ApplicationController
    AUDIT_LOGS_PER_PAGE = 50

    before_action :authorize_audit_access

    # GET /admin/audit_logs
    # Lists all audit log entries with optional filtering
    def index
      base_query = PaperTrail::Version.order(created_at: :desc)
      base_query = apply_filters_to_query(base_query)

      @versions = base_query
                  .limit(AUDIT_LOGS_PER_PAGE)
                  .offset((page_number - 1) * AUDIT_LOGS_PER_PAGE)
      @total_count = @versions.unscope(:limit, :offset).count

      render Components::Admin::AuditLogs::IndexView.new(
        versions: @versions,
        filter_params: params.slice(:item_type, :event, :whodunnit),
        current_page: page_number,
        total_count: @total_count,
        per_page: AUDIT_LOGS_PER_PAGE
      )
    end

    # GET /admin/audit_logs/:id
    # Shows detailed information about a specific audit log entry
    def show
      @version = PaperTrail::Version.find(params[:id])
      authorize @version, policy_class: AuditLogPolicy

      render Components::Admin::AuditLogs::ShowView.new(version: @version)
    end

    private

    def authorize_audit_access
      authorize :audit_log, :index?, policy_class: AuditLogPolicy
    end

    # Ensures page number is at least 1
    def page_number
      [params[:page].to_i, 1].max
    end

    # Applies filter parameters to the versions query
    # @param query [ActiveRecord::Relation] Base query to filter
    # @return [ActiveRecord::Relation] Filtered query
    def apply_filters_to_query(query)
      query = query.where(item_type: params[:item_type]) if params[:item_type].present?
      query = query.where(event: params[:event]) if params[:event].present?
      query = query.where(whodunnit: params[:whodunnit]) if params[:whodunnit].present?
      query
    end
  end
end
