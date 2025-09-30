# frozen_string_literal: true

# Controller for handling user registration and authentication
class UsersController < ApplicationController
  skip_before_action :require_authentication, only: %i[new create]
  def new
    @user = User.new
    @user.build_person
  end

  def create
    @user = User.new(user_params)

    if @user.save
      start_new_session_for(@user)
      redirect_to root_path, notice: t('users.welcome')
    else
      @user.build_person unless @user.person
      render :new, status: :unprocessable_entity
    end
  end

  private

  def user_params
    params.expect(user: [:email_address, :password, :password_confirmation,
                         { person_attributes: %i[name date_of_birth] }])
  end
end
