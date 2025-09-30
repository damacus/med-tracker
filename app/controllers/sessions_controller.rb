# frozen_string_literal: true

# Controller for handling user sessions
class SessionsController < ApplicationController
  allow_unauthenticated_access only: %i[new create]
  rate_limit to: 10, within: 3.minutes, only: :create, with: lambda {
    redirect_to new_session_url, alert: t('sessions.rate_limit')
  }

  def new
    render Views::Sessions::New.new(
      params: params,
      alert_message: flash[:alert],
      notice_message: flash[:notice]
    )
  end

  def create
    if (user = User.authenticate_by(params.permit(:email_address, :password)))
      start_new_session_for user
      redirect_to after_authentication_url, notice: t('sessions.signed_in')
    else
      # Preserve the email address parameter for re-display
      redirect_to new_session_path(email_address: params[:email_address]),
                  alert: t('sessions.invalid_credentials')
    end
  end

  def destroy
    terminate_session
    redirect_to login_path, notice: t('sessions.signed_out')
  end
end
