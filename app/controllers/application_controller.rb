# frozen_string_literal: true

class ApplicationController < ActionController::Base
  include Authentication
  include Pundit::Authorization

  before_action :set_paper_trail_whodunnit

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
    redirect_back(fallback_location: root_path)
  end
end
