# frozen_string_literal: true

# Controller for handling user sessions
class SessionsController < ApplicationController
  allow_unauthenticated_access only: %i[new create]
  rate_limit to: 10, within: 3.minutes, only: :create, with: lambda {
    redirect_to new_session_url, alert: 'Try again later.'
  }

  def new
    render Views::Sessions::New.new(params: params)
  end

  def create
    if (user = User.authenticate_by(params.permit(:email_address, :password)))
      start_new_session_for user
      redirect_to after_authentication_url, notice: 'Signed in successfully.'
    else
      # Preserve the email address parameter for re-display
      redirect_to new_session_path(email_address: params[:email_address]),
                  alert: 'Try another email address or password.'
    end
  end

  def destroy
    terminate_session
    redirect_to login_path, notice: 'Signed out successfully.'
  end
end
