# frozen_string_literal: true

class ApplicationController < ActionController::Base
  include Authentication
  include Pundit::Authorization

  rescue_from Pundit::NotAuthorizedError, with: :user_not_authorized

  # Only allow modern browsers supporting webp images, web push, badges, import maps, CSS nesting, and CSS :has.
  allow_browser versions: :modern

  private

  def user_not_authorized
    flash[:alert] = t('pundit.not_authorized', default: 'You are not authorized to perform this action.')
    redirect_back(fallback_location: root_path)
  end
end
