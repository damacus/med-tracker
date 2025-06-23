class UsersController < ApplicationController
  skip_before_action :require_authentication, only: [:new, :create]
  def new
    @user = User.new
  end

  def create
    @user = User.new(user_params)

    if @user.save
      start_new_session_for(@user)
      redirect_to root_path, notice: "Welcome! You have signed up successfully."
    else
      render :new, status: :unprocessable_entity
    end
  end

  private

  def user_params
    params.require(:user).permit(:name, :date_of_birth, :email_address, :password, :password_confirmation)
  end
end
