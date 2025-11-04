# frozen_string_literal: true

module Admin
  # Handles admin access to user management functionality.
  class UsersController < ApplicationController
    def index
      authorize User
      users = policy_scope(User)
      users = apply_search(users) if params[:search].present?
      users = apply_role_filter(users) if params[:role].present?
      users = users.order(:created_at)
      render Components::Admin::Users::IndexView.new(users: users, search_params: search_params)
    end

    def new
      @user = User.new
      @user.build_person
      authorize @user
      render Components::Admin::Users::FormView.new(user: @user, url_helpers: self)
    end

    def edit
      @user = User.find(params[:id])
      authorize @user
      render Components::Admin::Users::FormView.new(user: @user, url_helpers: self)
    end

    def create
      @user = User.new(user_params)
      authorize @user

      if @user.save
        redirect_to admin_users_path, notice: 'User was successfully created.'
      else
        render Components::Admin::Users::FormView.new(user: @user, url_helpers: self), status: :unprocessable_entity
      end
    end

    def update
      @user = User.find(params[:id])
      authorize @user

      if @user.update(user_params)
        redirect_to admin_users_path, notice: 'User was successfully updated.'
      else
        render Components::Admin::Users::FormView.new(user: @user, url_helpers: self), status: :unprocessable_entity
      end
    end

    private

    def apply_search(scope)
      search_term = "%#{params[:search]}%"
      scope.joins(:person)
           .where('people.name LIKE ? OR users.email_address LIKE ?', search_term, search_term)
    end

    def apply_role_filter(scope)
      scope.where(role: params[:role])
    end

    def search_params
      params.permit(:search, :role)
    end

    def user_params
      params.expect(
        user: [:email_address,
               :password,
               :password_confirmation,
               :role,
               { person_attributes: %i[id name date_of_birth] }]
      )
    end
  end
end
