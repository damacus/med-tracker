# frozen_string_literal: true

module Views
  module Rodauth
    class Login < Views::Rodauth::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::LinkTo
      include Views::Rodauth::LoginPasskeySupport

      def view_template
        div(class: 'min-h-screen bg-[var(--bg)] flex flex-col items-center justify-center p-6 transition-all duration-500') do
          render_background_atmosphere
          div(class: 'w-full max-w-[420px] relative z-10') do
            render_brand_identity
            render_login_card
            render_signup_prompt
          end
        end
      end

      private

      def render_background_atmosphere
        div(class: 'fixed inset-0 overflow-hidden pointer-events-none') do
          div(class: 'absolute -top-[10%] -left-[10%] h-[50%] w-[50%] rounded-full bg-primary/10 blur-[120px]')
          div(class: 'absolute -bottom-[10%] -right-[10%] h-[50%] w-[50%] rounded-full bg-accent opacity-70 blur-[120px]')
        end
      end

      def render_brand_identity
        div(class: 'flex flex-col items-start mb-10 px-2') do
          div(class: 'w-12 h-12 rounded-2xl bg-[var(--primary)] flex items-center justify-center text-white shadow-lg shadow-[var(--primary)]/20 mb-6 transition-transform hover:scale-105') do
            render Icons::Pill.new(size: 24)
          end
          Heading(level: 1, size: '7', class: 'font-extrabold tracking-tight text-[var(--text-main)]') { t('app.name') }
          Text(size: '2', weight: 'muted', class: 'mt-1 uppercase tracking-[0.2em] font-bold text-muted-foreground') do
            t('sessions.login.tagline')
          end
        end
      end

      def render_login_card
        render RubyUI::Card.new(
          class: 'overflow-hidden rounded-[2.5rem] border border-border/70 bg-card/90 shadow-[0_32px_64px_-12px_rgba(0,0,0,0.18)]'
        ) do
          div(class: 'p-10 sm:p-12') do
            render_login_header
            flash_section
            render_login_form
            render_passkey_section
            render_oauth_section
          end
        end
      end

      def render_login_header
        div(class: 'mb-10') do
          Heading(level: 2, size: '5', class: 'font-bold mb-1.5') { t('sessions.login.heading') }
          Text(size: '2', class: 'text-muted-foreground') { t('sessions.login.subheading') }
        end
      end

      def render_login_form
        render RubyUI::Form.new(action: view_context.rodauth.login_path, method: :post, class: 'space-y-7',
                                data_turbo: 'false') do
          authenticity_token_field
          render_identity_field
          render_credentials_field
          render_options_field
          render_submit_button
        end
      end

      def render_identity_field
        div(class: 'space-y-2.5') do
          render RubyUI::FormFieldLabel.new(for: 'email',
                                            class: 'ml-5 text-[10px] font-black uppercase tracking-widest text-muted-foreground') do
            t('sessions.login.email_label')
          end
          render RubyUI::Input.new(**email_input_attrs,
                                   class: 'rounded-2xl border-border bg-background/80 px-5 py-6 text-foreground transition-all placeholder:text-muted-foreground focus:bg-card focus:ring-4 focus:ring-[var(--primary)]/10 focus:border-[var(--primary)]')
        end
      end

      def render_credentials_field
        div(class: 'space-y-2.5') do
          div(class: 'flex items-center justify-between px-5') do
            render RubyUI::FormFieldLabel.new(for: 'password',
                                              class: 'text-[10px] font-black uppercase tracking-widest text-muted-foreground') do
              t('sessions.login.password_label')
            end
            render RubyUI::Link.new(href: view_context.rodauth.reset_password_request_path, variant: :link, size: :sm,
                                    class: 'text-[11px] font-bold text-[var(--primary)] p-0 h-auto hover:underline') do
              t('sessions.login.forgot_password')
            end
          end
          render RubyUI::Input.new(**password_input_attrs,
                                   class: 'rounded-2xl border-border bg-background/80 px-5 py-6 text-foreground transition-all placeholder:text-muted-foreground focus:bg-card focus:ring-4 focus:ring-[var(--primary)]/10 focus:border-[var(--primary)]')
        end
      end

      def render_options_field
        div(class: 'flex items-center px-5') do
          div(class: 'flex items-center gap-3 cursor-pointer group') do
            input(
              type: 'checkbox', name: 'remember', id: 'remember', value: 't',
              class: 'h-4 w-4 cursor-pointer rounded-md border-2 border-border bg-background text-[var(--primary)] transition-all focus:ring-[var(--primary)]'
            )
            label(for: 'remember',
                  class: 'cursor-pointer select-none text-sm font-semibold text-muted-foreground transition-colors group-hover:text-foreground') do
              t('sessions.login.remember_me')
            end
          end
        end
      end

      def render_submit_button
        div(class: 'pt-2') do
          render RubyUI::Button.new(type: :submit, variant: :primary, class: 'w-full py-8 font-black text-base') do
            t('sessions.login.submit')
          end
        end
      end

      def render_oauth_section
        return unless oauth_enabled?

        div(class: 'mt-10 border-t border-border pt-10') do
          provider_name = ENV.fetch('OIDC_PROVIDER_NAME', 'OIDC')
          form(action: view_context.rodauth.omniauth_request_path(:oidc), method: 'post', data: { turbo: 'false' }) do
            input(type: 'hidden', name: 'authenticity_token', value: view_context.form_authenticity_token)
            render RubyUI::Button.new(type: :submit, variant: :outline, class: 'w-full rounded-2xl border-border py-7 font-bold text-muted-foreground shadow-sm transition-all hover:bg-accent') do
              render_oidc_icon
              span { t('sessions.login.oauth_continue', provider: provider_name) }
            end
          end
        end
      end

      def render_signup_prompt
        div(class: 'mt-10 text-center space-y-4') do
          render_create_account_link unless invite_only?
          render_resend_verification_link
        end
      end

      def render_create_account_link
        Text(size: '2', weight: 'medium', class: 'text-muted-foreground') do
          plain "#{t('sessions.login.need_account')} "
          render RubyUI::Link.new(href: view_context.rodauth.create_account_path, variant: :link,
                                  class: 'font-bold text-[var(--primary)] p-0 h-auto hover:underline') do
            t('sessions.login.create_account')
          end
        end
      end

      def render_resend_verification_link
        div do
          render RubyUI::Link.new(href: view_context.rodauth.verify_account_resend_path, variant: :link,
                                  class: 'text-xs font-medium text-muted-foreground hover:text-foreground') do
            t('sessions.login.resend_verification')
          end
        end
      end

      def flash_section
        return if flash_message.blank?

        div(id: 'login-flash', class: 'mb-8') do
          render RubyUI::Alert.new(variant: flash_variant, class: 'rounded-2xl border-none shadow-sm text-center') do
            plain(flash_message)
          end
        end
      end

      def flash_message
        view_context.flash[:alert] || view_context.flash[:notice]
      end

      def flash_variant
        view_context.flash[:alert].present? ? :destructive : :success
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

      def render_oidc_icon
        render Icons::Globe.new(size: 20, class: 'mr-3')
      end

      def invite_only?
        User.administrator.exists?
      end
    end
  end
end
