# frozen_string_literal: true

module Admin
  # Handles admin access to user management functionality.
  class UsersController < ApplicationController
    def index
      authorize User
      users = policy_scope(User).includes(:person)
      users = apply_search(users) if params[:search].present?
      users = apply_role_filter(users) if params[:role].present?
      users = apply_status_filter(users) if params[:status].present?
      users = apply_sorting(users)
      @pagy, users = pagy(:offset, users)
      render Components::Admin::Users::IndexView.new(
        users: users,
        search_params: search_params,
        current_user: current_user,
        pagy: @pagy
      )
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
        redirect_to admin_users_path, notice: t('users.created')
      else
        render Components::Admin::Users::FormView.new(user: @user, url_helpers: self), status: :unprocessable_content
      end
    end

    def update
      @user = User.find(params[:id])
      authorize @user

      if @user.update(user_params)
        redirect_to admin_users_path, notice: t('users.updated')
      else
        render Components::Admin::Users::FormView.new(user: @user, url_helpers: self), status: :unprocessable_content
      end
    end

    def destroy
      @user = User.find(params[:id])
      authorize @user

      if @user == current_user
        redirect_to admin_users_path, alert: t('users.cannot_deactivate_self')
      else
        @user.deactivate!
        redirect_to admin_users_path, notice: t('users.deactivated')
      end
    end

    def activate
      @user = User.find(params[:id])
      authorize @user

      @user.activate!
      redirect_to admin_users_path, notice: t('users.activated')
    end

    private

    def apply_search(scope)
      search_term = "%#{ActiveRecord::Base.sanitize_sql_like(params[:search])}%"
      scope.joins(:person)
           .where('people.name ILIKE ? OR users.email_address ILIKE ?', search_term, search_term)
    end

    def apply_role_filter(scope)
      scope.where(role: params[:role])
    end

    def apply_status_filter(scope)
      case params[:status]
      when 'active' then scope.active
      when 'inactive' then scope.inactive
      else scope
      end
    end

    def apply_sorting(scope)
      sort_column = params[:sort].presence_in(allowed_sort_columns) || 'created_at'
      sort_direction = params[:direction].presence_in(%w[asc desc]) || 'asc'

      case sort_column
      when 'name'
        scope.left_joins(:person).order(Arel.sql("people.name #{sort_direction}"))
      when 'email'
        scope.order(email_address: sort_direction.to_sym)
      when 'role'
        scope.order(role: sort_direction.to_sym)
      else
        scope.order(created_at: sort_direction.to_sym)
      end
    end

    def allowed_sort_columns
      %w[name email created_at role]
    end

    def search_params
      params.permit(:search, :role, :status, :sort, :direction)
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
