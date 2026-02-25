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
      render Components::Admin::Users::FormView.new(user: @user)
    end

    def edit
      @user = User.find(params[:id])
      authorize @user
      render Components::Admin::Users::FormView.new(user: @user)
    end

    def create
      @user = User.new(user_params)
      authorize @user

      return render_user_form_with_errors if account_already_exists?

      create_user_with_account!
      redirect_to admin_users_path, notice: t('users.created')
    rescue ActiveRecord::RecordInvalid => e
      handle_record_invalid_error(e)
      render_user_form_with_errors
    end

    def update
      @user = User.find(params[:id])
      authorize @user

      if @user.update(user_params)
        redirect_to admin_users_path, notice: t('users.updated')
      else
        render Components::Admin::Users::FormView.new(user: @user), status: :unprocessable_content
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

    def verify
      @user = User.find(params[:id])
      authorize @user

      account = @user.person&.account
      return redirect_to admin_users_path, alert: t('admin.users.missing_account') unless account

      ActiveRecord::Base.transaction do
        account.update!(status: :verified)
        AccountVerificationKey.where(account_id: account.id).delete_all
      end

      redirect_to admin_users_path, notice: t('users.verified')
    end

    private

    def account_already_exists?
      return false unless Account.exists?(email: @user.email_address)

      @user.errors.add(:email_address, 'has already been taken')
      true
    end

    def create_user_with_account!
      ActiveRecord::Base.transaction do
        account = Account.create!(
          email: @user.email_address,
          password_hash: BCrypt::Password.create(params[:user][:password]),
          status: :verified
        )
        @user.person.account = account
        @user.save!
      end
    end

    def handle_record_invalid_error(exception)
      @user ||= User.new(user_params)
      return unless exception.record.is_a?(Account) && exception.record.errors[:email].any?

      exception.record.errors[:email].each do |error|
        @user.errors.add(:email_address, error)
      end
    end

    def render_user_form_with_errors
      render Components::Admin::Users::FormView.new(user: @user), status: :unprocessable_content
    end

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
               { person_attributes: [:id, :name, :date_of_birth, { location_ids: [] }] }]
      )
    end
  end
end
