# frozen_string_literal: true

module Views
  module Rodauth
    class Login < Views::Rodauth::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::LinkTo
      include Views::Rodauth::LoginPasskeySupport

      def view_template
        div(class: 'relative min-h-screen bg-[#fafafa] flex items-center justify-center p-4 sm:p-6 lg:p-8') do
          div(class: 'w-full max-w-[440px] space-y-12') do
            render_brand_header
            render_login_card
            render_footer_links
          end
        end
      end

      private

      def render_brand_header
        div(class: 'flex flex-col items-center space-y-4') do
          div(class: 'inline-flex h-12 w-12 items-center justify-center rounded-xl bg-zinc-950 text-white shadow-sm') do
            render Components::Icons::Pill.new(size: 24)
          end
          div(class: 'text-center space-y-1') do
            h1(class: 'text-xl font-bold tracking-tight text-zinc-900') { t('app.name') }
            p(class: 'text-sm text-zinc-500') { t('sessions.login.subheading') }
          end
        end
      end

      def render_login_card
        div(class: 'bg-white rounded-3xl border border-zinc-200/60 p-8 sm:p-10 shadow-[0_8px_30px_rgb(0,0,0,0.04)]') do
          div(class: 'space-y-8') do
            flash_section
            render_login_form
            render_passkey_section
            render_oauth_section
          end
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
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(for: 'email', class: 'text-xs font-semibold uppercase tracking-wider text-zinc-500') do
            t('sessions.login.email_label')
          end
          render RubyUI::Input.new(**email_input_attrs,
                                   class: 'h-12 rounded-xl border-zinc-200 bg-zinc-50/30 px-4 transition-all focus:border-zinc-950 focus:ring-zinc-950')
        end
      end

      def render_password_field
        div(class: 'space-y-2') do
          div(class: 'flex items-center justify-between') do
            render RubyUI::FormFieldLabel.new(for: 'password', class: 'text-xs font-semibold uppercase tracking-wider text-zinc-500') do
              t('sessions.login.password_label')
            end
            render RubyUI::Link.new(href: view_context.rodauth.reset_password_request_path, variant: :link, size: :sm,
                                    class: 'h-auto p-0 text-xs font-medium text-zinc-500 hover:text-zinc-950') do
              t('sessions.login.forgot_password')
            end
          end
          render RubyUI::Input.new(**password_input_attrs,
                                   class: 'h-12 rounded-xl border-zinc-200 bg-zinc-50/30 px-4 transition-all focus:border-zinc-950 focus:ring-zinc-950')
        end
      end

      def render_form_options
        div(class: 'flex items-center justify-between pt-2') do
          div(class: 'flex items-center gap-2') do
            input(
              type: 'checkbox', name: 'remember', id: 'remember', value: 't',
              class: 'h-4 w-4 rounded border-zinc-300 text-zinc-950 focus:ring-zinc-950'
            )
            label(for: 'remember', class: 'text-sm text-zinc-600') do
              t('sessions.login.remember_me')
            end
          end
        end
      end

      def render_submit_button
        render RubyUI::Button.new(type: :submit, variant: :primary,
                                  class: 'w-full h-12 rounded-xl bg-zinc-950 font-semibold text-white transition-all hover:bg-zinc-800 active:scale-[0.98]') do
          t('sessions.login.submit')
        end
      end

      def render_footer_links
        div(class: 'text-center space-y-4 pt-4') do
          render_signup_link
          render_resend_link
        end
      end

      def render_signup_link
        return if invite_only?

        p(class: 'text-sm text-zinc-600') do
          plain "#{t('sessions.login.need_account')} "
          render RubyUI::Link.new(href: view_context.rodauth.create_account_path, variant: :link,
                                  class: 'p-0 h-auto font-semibold text-zinc-950 hover:underline') do
            t('sessions.login.create_account')
          end
        end
      end

      def render_resend_link
        div do
          render RubyUI::Link.new(href: view_context.rodauth.verify_account_resend_path, variant: :link,
                                  class: 'text-xs font-medium text-zinc-400 hover:text-zinc-950') do
            t('sessions.login.resend_verification')
          end
        end
      end

      def render_oauth_section
        return unless oauth_enabled?

        div(class: 'relative mt-8') do
          render_oauth_divider
          render_oauth_button
        end
      end

      def render_oauth_divider
        div(class: 'absolute inset-0 flex items-center', aria_hidden: 'true') do
          div(class: 'w-full border-t border-zinc-100')
        end
        div(class: 'relative flex justify-center text-xs uppercase tracking-widest') do
          span(class: 'bg-white px-2 text-zinc-400') { 'or continue with' }
        end
      end

      def render_oauth_button
        div(class: 'mt-14') do # Adjusted margin to account for divider
          provider_name = ENV.fetch('OIDC_PROVIDER_NAME', 'OIDC')
          form(action: view_context.rodauth.omniauth_request_path(:oidc), method: 'post', data: { turbo: 'false' }) do
            input(type: 'hidden', name: 'authenticity_token', value: view_context.form_authenticity_token)
            render RubyUI::Button.new(type: :submit, variant: :outline,
                                      class: 'w-full h-12 rounded-xl border-zinc-200 bg-white font-medium text-zinc-700 transition-all hover:bg-zinc-50 hover:border-zinc-300') do
              render Components::Icons::Globe.new(size: 18, class: 'mr-2 text-zinc-400')
              span { provider_name }
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

        render RubyUI::Alert.new(variant: flash_variant, class: 'mb-6 rounded-xl border-zinc-100') do
          plain(flash_message)
        end
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
