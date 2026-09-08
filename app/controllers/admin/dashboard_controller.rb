# frozen_string_literal: true

module Admin
  # Handles admin dashboard functionality
  class DashboardController < BaseController
    def index
      authorize :admin_dashboard, :index?

      can_import_dmd = policy(:admin_nhs_dmd_import).new?
      metrics = Admin::DashboardMetricsQuery.new(import_dmd_allowed: can_import_dmd).call

      render Components::Admin::Dashboard::IndexView.new(metrics: metrics, import_dmd_allowed: can_import_dmd)
    end
  end
end
