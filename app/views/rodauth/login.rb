# frozen_string_literal: true

module Views
  module Rodauth
    class Login < Views::Rodauth::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::LinkTo
      include Views::Rodauth::LoginBrandSupport
      include Views::Rodauth::LoginBenefitsSupport
      include Views::Rodauth::LoginFormSupport
      include Views::Rodauth::LoginInputAttrsSupport
      include Views::Rodauth::LoginLayoutSupport
      include Views::Rodauth::LoginPasskeySupport
      include Views::Rodauth::LoginSecondarySignInSupport

      def view_template
        login_page_layout do
          render_login_card
          render_footer_links
        end
      end

      private

      def render_login_card
        div(**login_surface_attributes) do
          render_brand_panel
          render_form_panel
        end
      end

      def render_footer_links
        div(class: 'w-full max-w-5xl text-center space-y-4') do
          render_signup_link
          render_resend_link
          render_version_display
        end
      end

      def render_signup_link
        return if invite_only?

        m3_text(variant: :body_medium, class: 'text-on-surface-variant font-medium') do
          plain "#{t('sessions.login.need_account')} "
          m3_link(href: view_context.rodauth.create_account_path, variant: :text, class: 'p-0 h-auto font-black underline') do
            t('sessions.login.create_account')
          end
        end
      end

      def render_resend_link
        div do
          m3_link(href: view_context.rodauth.verify_account_resend_path, variant: :text, size: :sm, class: 'font-bold opacity-70 hover:opacity-100') do
            t('sessions.login.resend_verification')
          end
        end
      end

      def render_version_display
        span(class: 'block text-[10px] text-on-surface-variant/50 font-mono font-bold uppercase tracking-widest') do
          "v#{ENV.fetch('APP_VERSION', MedTracker::VERSION)}"
        end
      end

      def oauth_enabled?
        oidc_configured?
      end

      def oidc_configured?
        return false unless view_context.rodauth.respond_to?(:omniauth_request_path)

        oidc_client_id = Rails.application.credentials.dig(:oidc, :client_id) || ENV.fetch('OIDC_CLIENT_ID', nil)
        oidc_issuer = Rails.application.credentials.dig(:oidc, :issuer_url) || ENV.fetch('OIDC_ISSUER_URL', nil)
        oidc_client_id.present? && oidc_issuer.present?
      end

      def invite_only?
        User.administrator.exists?
      end

      def flash_section
        return if flash_message.blank?

        render_m3_alert(flash_message, variant: flash_variant)
      end

      def flash_message
        message = view_context.flash[:alert] || view_context.flash[:notice]
        return if message == I18n.t('authentication.login_required', default: 'Please login to continue')

        message
      end

      def flash_variant
        view_context.flash[:alert].present? ? :destructive : :success
      end
    end
  end
end
