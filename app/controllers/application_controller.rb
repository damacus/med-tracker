# frozen_string_literal: true

class ApplicationController < ActionController::Base
  include Authentication
  include Pundit::Authorization
  include Pagy::Method
  include TurboNativeDetectable
  include TenantDomTargetsHelper

  before_action :set_paper_trail_whodunnit
  around_action :with_current_context
  after_action :verify_pundit_authorization, if: :pundit_verification_required?
  helper_method :current_household, :current_membership

  rescue_from ActionController::InvalidAuthenticityToken, with: :handle_invalid_authenticity_token
  rescue_from Pundit::NotAuthorizedError, with: :user_not_authorized

  # Only allow modern browsers supporting webp images, web push, badges, import maps, CSS nesting, and CSS :has.
  allow_browser versions: :modern

  def default_url_options
    options = super
    household = Current.household || default_household_for_urls
    return options unless household

    options.merge(household_slug: household.slug)
  end

  private

  attr_reader :current_household, :current_membership

  def pundit_user
    AuthorizationContext.current || support_authorization_context || current_user
  end

  def verify_pundit_authorization
    verify_authorized
  end

  def pundit_verification_required?
    !request.get? && !request.head?
  end

  def with_current_context(&)
    Current.account = current_account
    Current.request_id = request.request_id
    Time.use_zone(current_account_time_zone) { with_current_tenant_context(&) }
  ensure
    Current.reset
  end

  def with_current_tenant_context
    household_slug = request.path_parameters[:household_slug]
    return yield if household_slug.blank?

    @current_household = Household.find_by!(slug: household_slug)
    TenantContext.with(account: current_account, household: @current_household, request_id: request.request_id) do
      @current_membership = active_household_membership

      if @current_membership
        TenantContext.set_membership!(@current_membership)
        PaperTrail.request.controller_info = info_for_paper_trail
        yield
      elsif active_support_access_session
        @current_support_access_session = active_support_access_session
        Current.support_access_session = @current_support_access_session
        PaperTrail.request.controller_info = info_for_paper_trail
        yield
      else
        user_not_authorized
      end
    end
  end

  def current_account_time_zone
    current_account&.preferred_time_zone || Rails.application.config.time_zone
  end

  def active_household_membership
    return unless current_account

    current_account.household_memberships.active.find_by(household: @current_household)
  end

  def active_support_access_session
    return @active_support_access_session if defined?(@active_support_access_session)
    return unless current_account&.platform_admin

    @active_support_access_session =
      current_account.platform_admin.support_access_sessions.active.find_by(household: @current_household)
  end

  def default_household_for_urls
    return @default_household_for_urls if defined?(@default_household_for_urls)

    @default_household_for_urls = current_account&.first_active_household
  end

  def user_for_paper_trail
    current_user&.id
  end

  def info_for_paper_trail
    {
      ip: request.remote_ip,
      request_id: request.request_id,
      household_id: Current.household&.id,
      actor_membership_id: Current.membership&.id
    }
  end

  def support_authorization_context
    return unless Current.account && Current.household && Current.support_access_session

    AuthorizationContext.new(account: Current.account, household: Current.household, membership: nil)
  end

  def modal_frame_request?
    request.headers['Turbo-Frame'] == 'modal'
  end

  def render_modal_or_page(modal:, page:, status: :ok)
    respond_to do |format|
      format.html do
        if modal_frame_request?
          render modal_renderable(modal), layout: false, status: status
        else
          render modal_renderable(page), status: status
        end
      end

      format.turbo_stream do
        render turbo_stream: turbo_stream.replace('modal', modal_renderable(modal)), status: status
      end
    end
  end

  def modal_renderable(renderable)
    renderable.is_a?(Proc) ? renderable.call : renderable
  end

  def safe_redirect_path(path)
    url_from(path)
  end
  helper_method :safe_redirect_path

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
      format.any do
        redirect_to rodauth.login_path,
                    alert: session_expired_message,
                    status: :see_other
      end
    end
  end
end
