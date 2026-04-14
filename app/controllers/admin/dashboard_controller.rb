# frozen_string_literal: true

module Admin
  # Handles admin dashboard functionality
  class DashboardController < ApplicationController
    def index
      authorize :admin_dashboard, :index?

      metrics = Admin::DashboardMetricsQuery.new.call

      render Components::Admin::Dashboard::IndexView.new(metrics: metrics)
    end
  end
end
