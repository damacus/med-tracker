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
      @versions = PaperTrail::Version
                  .order(created_at: :desc)
                  .limit(AUDIT_LOGS_PER_PAGE)
                  .offset((page_number - 1) * AUDIT_LOGS_PER_PAGE)

      apply_filters

      render Components::Admin::AuditLogs::IndexView.new(
        versions: @versions,
        filter_params: params.slice(:item_type, :event, :whodunnit)
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
    def apply_filters
      @versions = @versions.where(item_type: params[:item_type]) if params[:item_type].present?
      @versions = @versions.where(event: params[:event]) if params[:event].present?
      @versions = @versions.where(whodunnit: params[:whodunnit]) if params[:whodunnit].present?
    end
  end
end
