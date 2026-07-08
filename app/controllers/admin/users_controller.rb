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

      unless create_membership_role_valid?
        respond_to do |format|
          format.html { render_user_form_with_errors }
          format.turbo_stream { render_user_form_with_errors }
        end
        return
      end

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
      @load_locations ||= policy_scope(Location).to_a
    end

    def load_dependents
      @load_dependents ||=
        dependent_assignment_scope.where(person_type: %i[minor dependent_adult], has_capacity: false).order(:name).to_a
    end

    def account_already_exists?
      return false unless Account.exists?(email: @user.email_address)

      @user.errors.add(:email_address, :taken)
      true
    end

    def create_membership_role_valid?
      requested_role = @user.membership_role.presence || 'member'
      return true if ::Admin::MembershipRoleUpdater::ALLOWED_ROLES.include?(requested_role)

      @user.errors.add(:membership_role, create_membership_role_error(requested_role))
      false
    end

    def create_membership_role_error(requested_role)
      if requested_role == ::Admin::MembershipRoleUpdater::OWNER_ROLE
        t('admin.membership_roles.owner_rejected')
      else
        t('admin.membership_roles.invalid_role')
      end
    end

    def create_user_with_account!
      ActiveRecord::Base.transaction do
        account = Account.create!(
          email: @user.email_address,
          password_hash: BCrypt::Password.create(params[:user][:password]),
          status: :verified
        )
        @user.person.account = account
        @user.person.household ||= admin_target_household
        @user.save!
        membership = create_household_membership_for(account, @user.person)
        grant_person_access(membership, @user.person, :manage, :self)
        assign_dependents(membership)
      end
    end

    def update_user_profile
      ActiveRecord::Base.transaction do
        @user.update(user_update_params)
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
      Components::Admin::Users::IndexView.new(
        users: users,
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

    def assign_dependents(membership = nil)
      return if carer_relationship_type.blank?

      relationships = DependentRelationshipAssigner.new(
        carer: @user.person,
        dependent_ids: @user.dependent_ids,
        relationship_type: carer_relationship_type,
        scope: dependent_assignment_scope
      ).call
      relationships.each do |relationship|
        grant_person_access(membership, relationship.patient, dependent_access_level, dependent_relationship_type)
      end
    end

    def admin_target_household
      current_household || default_household_for_urls
    end

    def admin_target_membership
      current_membership || current_account&.active_household_membership_for(admin_target_household)
    end

    def create_household_membership_for(account, person)
      return unless admin_target_household

      admin_target_household.household_memberships.create!(
        account: account,
        person: person,
        role: membership_role,
        status: :active
      )
    end

    def household_membership_for(user)
      return unless admin_target_household

      membership = admin_target_household.household_memberships.active.find_or_initialize_by(account: user.person.account)
      membership.person = user.person
      membership.role = membership_role
      membership.status = :active
      membership.save!
      membership
    end

    def membership_role
      requested_role = @user.membership_role.presence
      return requested_role if ::Admin::MembershipRoleUpdater::ALLOWED_ROLES.include?(requested_role)

      :member
    end

    def dependent_assignment_scope
      return Person.none unless admin_target_household

      Person.where(household: admin_target_household)
    end

    def grant_person_access(membership, person, access_level, relationship_type)
      return unless admin_target_household && membership && person&.household_id == admin_target_household.id

      grant = admin_target_household.person_access_grants.find_or_initialize_by(
        household_membership: membership,
        person: person
      )
      grant.access_level = access_level
      grant.relationship_type = relationship_type
      grant.granted_by_membership ||= admin_target_membership || membership
      grant.revoked_at = nil
      grant.save!
    end

    def dependent_access_level
      requested_level = @user.dependent_access_level.presence
      return requested_level if PersonAccessGrant.access_levels.key?(requested_level)

      dependent_relationship_type == 'parent' ? :manage : :record
    end

    def dependent_relationship_type
      requested_type = @user.dependent_relationship_type.presence
      return requested_type if PersonAccessGrant.relationship_types.key?(requested_type) && requested_type != 'self'

      :professional
    end

    def carer_relationship_type
      case dependent_relationship_type.to_s
      when 'parent'
        'parent'
      when 'family_member'
        'family_member'
      else
        'professional_carer'
      end
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
