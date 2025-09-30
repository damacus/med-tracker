# frozen_string_literal: true

module Admin
  # Handles admin access to user management functionality.
  class UsersController < ApplicationController
    before_action :require_admin

    def index
      users = User.order(:created_at)
      render Components::Admin::Users::IndexView.new(users: users)
    end

    private

    def require_admin
      return if current_user&.admin?

      redirect_to root_path, alert: t('admin.users.unauthorized')
    end
  end
end
