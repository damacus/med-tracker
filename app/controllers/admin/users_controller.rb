# frozen_string_literal: true

module Admin
  # Handles admin access to user management functionality.
  class UsersController < BaseController
    def index
      authorize User
      users = Admin::UsersIndexQuery.new(
        scope: policy_scope(User),
        filters: search_params.to_h.symbolize_keys,
        household: admin_target_household
      ).call
      @pagy, users = pagy(:offset, users)
      access_summary = user_access_summary(users)
      render Components::Admin::Users::IndexView.new(
        users: users,
        access_summary: access_summary,
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

      respond_to do |format|
        if provision_user.success?
          format.html { redirect_to admin_users_path, notice: t('users.created') }
          format.turbo_stream { render_users_index_turbo(t('users.created')) }
        else
          format.html { render_user_form_with_errors }
          format.turbo_stream { render_user_form_with_errors }
        end
      end
    end

    def update
      @user = policy_scope(User).find(params.expect(:id))
      authorize @user

      respond_to do |format|
        if update_user_profile
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
      respond_to_user_lifecycle(run_user_lifecycle(:deactivate))
    end

    def activate
      @user = policy_scope(User).find(params.expect(:id))
      authorize @user
      respond_to_user_lifecycle(run_user_lifecycle(:activate))
    end

    def verify
      @user = policy_scope(User).find(params.expect(:id))
      authorize @user
      respond_to_user_lifecycle(run_user_lifecycle(:verify))
    end

    private

    def load_locations
      @load_locations ||= policy_scope(Location).to_a
    end

    def load_dependents
      @load_dependents ||=
        dependent_assignment_scope.where(person_type: %i[minor dependent_adult], has_capacity: false).order(:name).to_a
    end

    def provision_user
      Admin::UserProvisioner.new(
        user: @user,
        password: params.dig(:user, :password),
        household: admin_target_household,
        actor_membership: admin_target_membership
      ).call
    end

    def update_user_profile
      @user.update(user_update_params)
    end

    def render_user_form_with_errors
      render user_form_view, status: :unprocessable_content
    end

    def user_form_view
      apply_user_access_defaults
      Components::Admin::Users::FormView.new(
        user: @user,
        locations: load_locations,
        dependents: load_dependents
      )
    end

    def apply_user_access_defaults
      @user.membership_role ||= household_membership_for_form&.role || 'member'
      @user.dependent_access_level ||= 'record'
    end

    def household_membership_for_form
      account = @user.person&.account
      return unless admin_target_household && account

      admin_target_household.household_memberships.active.find_by(account: account)
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
        filters: search_params.to_h.symbolize_keys,
        household: admin_target_household
      ).call
      @pagy, users = pagy(:offset, users)
      access_summary = user_access_summary(users)
      Components::Admin::Users::IndexView.new(
        users: users,
        access_summary: access_summary,
        search_params: search_params,
        current_user: current_user,
        pagy: @pagy
      )
    end

    def search_params
      params.permit(:search, :membership_role, :status, :sort, :direction)
    end

    def user_params
      params.expect(
        user: [
          :email_address,
          :password,
          :password_confirmation,
          :membership_role,
          :dependent_access_level,
          :dependent_relationship_type,
          {
            dependent_ids: [],
            person_attributes: [:id, :name, :date_of_birth, { location_ids: [] }]
          }
        ]
      )
    end

    def user_update_params
      params.expect(
        user: [
          :email_address,
          {
            person_attributes: [:id, :name, :date_of_birth, { location_ids: [] }]
          }
        ]
      )
    end

    def admin_target_household
      current_household || default_household_for_urls
    end

    def admin_target_membership
      current_membership || current_account&.active_household_membership_for(admin_target_household)
    end

    def run_user_lifecycle(action)
      Admin::UserLifecycleUpdater.new(
        user: @user,
        action: action,
        actor: current_user,
        household: admin_target_household,
        actor_membership: admin_target_membership
      ).call
    end

    def dependent_assignment_scope
      return Person.none unless admin_target_household

      Person.where(household: admin_target_household)
    end

    def respond_to_user_lifecycle(result)
      respond_to do |format|
        if result.success?
          format.html { redirect_to admin_users_path, notice: result.message }
          format.turbo_stream { render_user_lifecycle_success(result.message) }
        else
          format.html { redirect_to admin_users_path, alert: result.message }
          format.turbo_stream { render_user_lifecycle_failure(result.message) }
        end
      end
    end

    def render_user_lifecycle_success(message)
      flash.now[:notice] = message
      render turbo_stream: user_row_streams(@user)
    end

    def render_user_lifecycle_failure(message)
      flash.now[:alert] = message
      render turbo_stream: turbo_stream.update('flash', Components::Layouts::Flash.new(alert: flash[:alert])),
             status: :unprocessable_content
    end

    def user_row_streams(user)
      rendered_user = User.includes(person: :account).find(user.id)
      access_summary = user_access_summary([rendered_user])
      [
        turbo_stream.replace(
          "user_#{user.id}",
          Components::Admin::Users::UserRow.new(
            user: rendered_user,
            current_user: current_user,
            access_summary: access_summary
          )
        ),
        turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
      ]
    end

    def user_access_summary(users)
      Admin::UserAccessSummaryQuery.new(users: users, household: admin_target_household).call
    end
  end
end
