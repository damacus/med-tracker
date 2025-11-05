# frozen_string_literal: true

module Admin
  # Controller for viewing audit logs
  class AuditLogsController < ApplicationController
    def index
      authorize AuditLog
      @audit_logs = policy_scope(AuditLog).recent.limit(100)
      render Components::Admin::AuditLogs::IndexView.new(audit_logs: @audit_logs)
    end
  end
end
