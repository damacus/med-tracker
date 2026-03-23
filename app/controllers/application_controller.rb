# frozen_string_literal: true

class ApplicationController < ActionController::Base
  include Authentication
  include Pundit::Authorization
  include Pagy::Method
  include TurboNativeDetectable

  before_action :set_paper_trail_whodunnit

  rescue_from ActionController::InvalidAuthenticityToken, with: :handle_invalid_authenticity_token
  rescue_from Pundit::NotAuthorizedError, with: :user_not_authorized

  # Only allow modern browsers supporting webp images, web push, badges, import maps, CSS nesting, and CSS :has.
  allow_browser versions: :modern

  private

  def user_for_paper_trail
    current_user&.id
  end

  def info_for_paper_trail
    { ip: request.remote_ip }
  end

  def user_not_authorized
    flash[:alert] = t('pundit.not_authorized', default: 'You are not authorized to perform this action.')
    redirect_back_or_to(root_path)
  end

  def handle_invalid_authenticity_token
    reset_session
    session_expired_message = t('authentication.session_expired')

    respond_to do |format|
      format.json do
        render json: { error: session_expired_message }, status: :unauthorized
      end
      format.turbo_stream do
        redirect_to rodauth.login_path,
                    alert: session_expired_message,
                    status: :see_other
      end
      format.html do
        redirect_to rodauth.login_path,
                    alert: session_expired_message,
                    status: :see_other
      end
      format.any do
        redirect_to rodauth.login_path,
                    alert: session_expired_message,
                    status: :see_other
      end
    end
  end
end
