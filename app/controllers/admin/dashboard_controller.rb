# frozen_string_literal: true

module Admin
  # Handles admin dashboard functionality
  class DashboardController < ApplicationController
    def index
      authorize :admin_dashboard, :index?
      render Components::Admin::Dashboard::IndexView.new
    end
  end
end
