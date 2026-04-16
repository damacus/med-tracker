# frozen_string_literal: true

module Views
  module Rodauth
    class Login < Views::Rodauth::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::LinkTo
      include Views::Rodauth::LoginPasskeySupport

      def view_template
        page_layout do
          render_page_header(
            title: t('app.name'),
            subtitle: t('sessions.login.subheading')
          )
          render_login_card
          render_footer_links
        end
      end

      private

      def render_login_card
        render_auth_card(
          title: t('sessions.login.heading'),
          subtitle: t('sessions.login.subheading')
        ) do
          div(id: 'login-flash') { flash_section }
          render_login_form
          render_passkey_section
          render_oauth_section
        end
      end

      def render_login_form
        render RubyUI::Form.new(action: view_context.rodauth.login_path, method: :post, class: 'space-y-6',
                                data_turbo: 'false') do
          authenticity_token_field
          render_email_field
          render_password_field
          render_form_options
          render_submit_button
        end
      end

      def render_email_field
        render_m3_form_field(
          label: t('sessions.login.email_label'),
          input_attrs: email_input_attrs,
          error: view_context.rodauth.field_error('email') || view_context.rodauth.field_error('login')
        )
      end

      def render_password_field
        render_m3_form_field(
          label: t('sessions.login.password_label'),
          input_attrs: password_input_attrs,
          error: view_context.rodauth.field_error('password'),
          actions: lambda {
            m3_link(href: view_context.rodauth.reset_password_request_path, variant: :text, size: :sm,
                    class: 'h-auto p-0 text-xs font-bold') do
              t('sessions.login.forgot_password')
            end
          }
        )
      end

      def render_form_options
        div(class: 'flex items-center justify-between pt-2 px-1') do
          div(class: 'flex items-center gap-2') do
            input(
              type: 'checkbox', name: 'remember', id: 'remember', value: 't',
              class: 'h-4 w-4 rounded border-outline-variant bg-surface-container-lowest text-primary focus:ring-primary'
            )
            label(for: 'remember', class: 'text-sm text-on-surface-variant font-medium') do
              t('sessions.login.remember_me')
            end
          end
        end
      end

      def render_submit_button
        render_m3_submit_button(t('sessions.login.submit'))
      end

      def render_footer_links
        div(class: 'w-full max-w-xl text-center space-y-4 pt-4') do
          render_signup_link
          render_resend_link
          render_invite_only_oidc_link if invite_only? && oauth_enabled?
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

      def render_oauth_section
        return unless oauth_enabled?
        return if invite_only?

        div(class: 'relative mt-10') do
          render_oauth_divider
          render_oauth_button
        end
      end

      def render_oauth_divider
        div(class: 'absolute inset-0 flex items-center', aria_hidden: 'true') do
          div(class: 'w-full border-t border-outline-variant/50')
        end
        div(class: 'relative flex justify-center text-[10px] uppercase tracking-[0.2em] font-black') do
          span(class: 'bg-surface-container-high px-4 text-on-surface-variant/60') { t('sessions.login.oauth_divider') }
        end
      end

      def render_oauth_button
        div(class: 'mt-8') do
          provider_name = ENV.fetch('OIDC_PROVIDER_NAME', 'OIDC')
          form(action: view_context.rodauth.omniauth_request_path(:oidc), method: 'post', data: { turbo: 'false' }) do
            input(type: 'hidden', name: 'authenticity_token', value: view_context.form_authenticity_token)
            m3_button(type: :submit, variant: :outlined, size: :lg,
                      class: 'w-full py-6 rounded-xl font-bold bg-surface-container-low') do
              render Components::Icons::Globe.new(size: 20, class: 'mr-2 text-primary')
              span { "Continue with #{provider_name}" }
            end
          end
        end
      end

      def render_invite_only_oidc_link
        provider_name = ENV.fetch('OIDC_PROVIDER_NAME', 'OIDC')
        div(class: 'space-y-2 text-xs text-on-surface-variant font-medium') do
          p { t('sessions.login.invite_only_oidc_notice', default: 'Single sign-on is reserved for invited accounts.') }
          form(action: view_context.rodauth.omniauth_request_path(:oidc), method: 'post', data: { turbo: 'false' }, class: 'inline-block') do
            input(type: 'hidden', name: 'authenticity_token', value: view_context.form_authenticity_token)
            m3_button(type: :submit, variant: :text, size: :sm, class: 'h-auto p-0 font-bold underline') do
              plain t('sessions.login.invite_only_oidc_cta', default: "Continue with #{provider_name}")
            end
          end
        end
      end

      def email_input_attrs
        {
          type: :email,
          name: 'email',
          id: 'email',
          required: true,
          autofocus: true,
          autocomplete: 'username webauthn',
          placeholder: t('sessions.login.email_placeholder'),
          value: view_context.params[:email]
        }
      end

      def password_input_attrs
        {
          type: :password,
          name: 'password',
          id: 'password',
          required: true,
          autocomplete: 'current-password',
          placeholder: t('sessions.login.password_placeholder'),
          maxlength: 72
        }
      end

      def oauth_enabled?
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
        view_context.flash[:alert] || view_context.flash[:notice]
      end

      def flash_variant
        view_context.flash[:alert].present? ? :destructive : :success
      end
    end
  end
end
