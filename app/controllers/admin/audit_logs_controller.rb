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
      result = Admin::AuditLogsQuery.new(
        scope: PaperTrail::Version.all,
        filters: params.permit(:item_type, :event, :whodunnit).to_h.symbolize_keys,
        page: params[:page],
        per_page: AUDIT_LOGS_PER_PAGE
      ).call

      @total_count = result.total_count
      @versions = result.versions

      render Components::Admin::AuditLogs::IndexView.new(
        versions: @versions,
        filter_params: params.permit(:item_type, :event, :whodunnit),
        current_page: result.page,
        total_count: @total_count,
        per_page: result.per_page
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
  end
end
