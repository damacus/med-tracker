class Admin::UsersController < ApplicationController
  before_action :require_admin

  def index
    @users = User.all
  end

  private

  def require_admin
    return if Current.user.admin?

    redirect_to root_path, alert: "You are not authorized to perform this action."
  end
end
