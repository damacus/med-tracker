# frozen_string_literal: true

module Admin
  # Handles admin access to user management functionality.
  class UsersController < ApplicationController
    def index
      authorize User
      users = Admin::UsersIndexQuery.new(
        scope: policy_scope(User),
        filters: search_params.to_h.symbolize_keys
      ).call
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
      render user_form_view
    end

    def edit
      @user = policy_scope(User).find(params.expect(:id))
      authorize @user
      render user_form_view
    end

    def create
      @user = User.new(user_params)
      authorize @user

      if account_already_exists?
        respond_to do |format|
          format.html { render_user_form_with_errors }
          format.turbo_stream { render_user_form_with_errors }
        end
        return
      end

      create_user_with_account!
      respond_to do |format|
        format.html { redirect_to admin_users_path, notice: t('users.created') }
        format.turbo_stream { render_users_index_turbo(t('users.created')) }
      end
    rescue ActiveRecord::RecordInvalid => e
      handle_record_invalid_error(e)
      respond_to do |format|
        format.html { render_user_form_with_errors }
        format.turbo_stream { render_user_form_with_errors }
      end
    end

    def update
      @user = policy_scope(User).find(params.expect(:id))
      authorize @user

      respond_to do |format|
        if update_user_with_dependents
          format.html { redirect_to admin_users_path, notice: t('users.updated') }
          format.turbo_stream { render_users_index_turbo(t('users.updated')) }
        else
          format.html { render_user_form_with_errors }
          format.turbo_stream { render_user_form_with_errors }
        end
      end
    end

    def destroy
      @user = policy_scope(User).find(params.expect(:id))
      authorize @user

      respond_to do |format|
        if @user == current_user
          format.html { redirect_to admin_users_path, alert: t('users.cannot_deactivate_self') }
          format.turbo_stream do
            flash.now[:alert] = t('users.cannot_deactivate_self')
            render turbo_stream: turbo_stream.update('flash', Components::Layouts::Flash.new(alert: flash[:alert])),
                   status: :unprocessable_content
          end
        else
          @user.deactivate!
          format.html { redirect_to admin_users_path, notice: t('users.deactivated') }
          format.turbo_stream do
            flash.now[:notice] = t('users.deactivated')
            render turbo_stream: user_row_streams(@user)
          end
        end
      end
    end

    def activate
      @user = policy_scope(User).find(params.expect(:id))
      authorize @user

      @user.activate!
      respond_to do |format|
        format.html { redirect_to admin_users_path, notice: t('users.activated') }
        format.turbo_stream do
          flash.now[:notice] = t('users.activated')
          render turbo_stream: user_row_streams(@user)
        end
      end
    end

    def verify
      @user = policy_scope(User).find(params.expect(:id))
      authorize @user

      account = @user.person&.account
      unless account
        respond_to do |format|
          format.html { redirect_to admin_users_path, alert: t('admin.users.missing_account') }
          format.turbo_stream do
            flash.now[:alert] = t('admin.users.missing_account')
            render turbo_stream: turbo_stream.update('flash', Components::Layouts::Flash.new(alert: flash[:alert])),
                   status: :unprocessable_content
          end
        end
        return
      end

      ActiveRecord::Base.transaction do
        account.update!(status: :verified)
        AccountVerificationKey.where(account_id: account.id).delete_all
      end

      respond_to do |format|
        format.html { redirect_to admin_users_path, notice: t('users.verified') }
        format.turbo_stream do
          flash.now[:notice] = t('users.verified')
          render turbo_stream: user_row_streams(@user)
        end
      end
    end

    private

    def load_locations
      @load_locations ||= Location.all.to_a
    end

    def load_dependents
      @load_dependents ||=
        Person.where(person_type: %i[minor dependent_adult], has_capacity: false).order(:name).to_a
    end

    def account_already_exists?
      return false unless Account.exists?(email: @user.email_address)

      @user.errors.add(:email_address, :taken)
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
        assign_dependents
      end
    end

    def update_user_with_dependents
      ActiveRecord::Base.transaction do
        updated = @user.update(user_params)
        assign_dependents if updated
        updated
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
      render user_form_view, status: :unprocessable_content
    end

    def user_form_view
      Components::Admin::Users::FormView.new(
        user: @user,
        locations: load_locations,
        dependents: load_dependents
      )
    end

    def render_users_index_turbo(message)
      flash.now[:notice] = message
      render turbo_stream: [
        turbo_stream.replace('main-content', admin_users_index_view),
        turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
      ]
    end

    def admin_users_index_view
      users = Admin::UsersIndexQuery.new(
        scope: policy_scope(User),
        filters: search_params.to_h.symbolize_keys
      ).call
      @pagy, users = pagy(:offset, users)
      Components::Admin::Users::IndexView.new(
        users: users,
        search_params: search_params,
        current_user: current_user,
        pagy: @pagy
      )
    end

    def search_params
      params.permit(:search, :role, :status, :sort, :direction)
    end

    def user_params
      params.expect(user: permitted_user_attributes)
    end

    def permitted_user_attributes
      attributes = %i[email_address password password_confirmation]
      attributes << :role if current_user.administrator?
      attributes << dependent_assignment_attributes
      attributes
    end

    def dependent_assignment_attributes
      attributes = { person_attributes: [:id, :name, :date_of_birth, { location_ids: [] }] }
      attributes[:dependent_ids] = [] if current_user.administrator?
      attributes
    end

    def assign_dependents
      return unless current_user.administrator?
      return unless DependentRelationshipAssigner.relationship_type_for(@user)

      DependentRelationshipAssigner.new(
        carer: @user.person,
        dependent_ids: @user.dependent_ids,
        relationship_type: DependentRelationshipAssigner.relationship_type_for(@user)
      ).call
    end

    def user_row_streams(user)
      [
        turbo_stream.replace(
          "user_#{user.id}",
          Components::Admin::Users::UserRow.new(user: user.reload, current_user: current_user)
        ),
        turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
      ]
    end
  end
end
