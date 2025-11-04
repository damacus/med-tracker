# frozen_string_literal: true

module Admin
  # Handles admin access to user management functionality.
  class UsersController < ApplicationController
    def index
      authorize User
      users = policy_scope(User).order(:created_at)
      render Components::Admin::Users::IndexView.new(users: users)
    end
  end
end
