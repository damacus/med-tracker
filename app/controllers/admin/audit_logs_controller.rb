# frozen_string_literal: true

module Admin
  # Controller for viewing audit trail logs
  # Read-only access to PaperTrail versions for administrators
  class AuditLogsController < ApplicationController
    before_action :authorize_audit_access

    def index
      @versions = PaperTrail::Version
                  .order(created_at: :desc)
                  .limit(50)
                  .offset((params[:page].to_i - 1) * 50)

      # Apply filters if provided
      @versions = @versions.where(item_type: params[:item_type]) if params[:item_type].present?
      @versions = @versions.where(event: params[:event]) if params[:event].present?
      @versions = @versions.where(whodunnit: params[:whodunnit]) if params[:whodunnit].present?
    end

    def show
      @version = PaperTrail::Version.find(params[:id])
      authorize @version, policy_class: AuditLogPolicy
    end

    private

    def authorize_audit_access
      authorize :audit_log, :index?, policy_class: AuditLogPolicy
    end
  end
end
