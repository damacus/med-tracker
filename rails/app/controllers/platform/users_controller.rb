# frozen_string_literal: true

module Platform
  class UsersController < BaseController
    def index
      users = User.includes(person: :account).order(:email_address)
      access_summary = Admin::UserAccessSummaryQuery.new(users: users).call
      render Components::Platform::Users::IndexView.new(
        users: users,
        current_user: current_user,
        access_summary: access_summary
      )
    end

    def update
      @user = User.includes(person: :account).find(params.expect(:id))
      authorize @user, :update?, policy_class: PlatformUserPolicy

      update_platform_access!
      redirect_to platform_users_path, notice: t('platform.users.updated')
    end

    private

    def update_platform_access!
      account = @user.person.account
      platform_admin = account.platform_admin || account.build_platform_admin
      platform_admin.status = system_administrator_requested? ? :active : :disabled
      platform_admin.save!
    end

    def system_administrator_requested?
      ActiveModel::Type::Boolean.new.cast(platform_user_params[:system_administrator])
    end

    def platform_user_params
      params.expect(platform_user: [:system_administrator])
    end
  end
end
