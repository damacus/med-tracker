# frozen_string_literal: true

module Views
  module Rodauth
    class Login < Views::Rodauth::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::LinkTo
      include Views::Rodauth::LoginHeroSupport
      include Views::Rodauth::LoginPasskeySupport
      include Views::Rodauth::LoginSupportRail

      def view_template
        div(class: 'relative min-h-screen overflow-hidden bg-[linear-gradient(135deg,#f4efe5_0%,#eef4f2_45%,#f8faf8_100%)] px-4 py-6 sm:px-6 lg:px-10') do
          render_background_atmosphere
          div(class: 'relative z-10 mx-auto flex min-h-[calc(100vh-3rem)] w-full max-w-7xl items-center justify-center') do
            div(class: 'grid w-full max-w-6xl gap-8 lg:grid-cols-2 lg:gap-10') do
              render_brand_panel
              render_login_shell
            end
          end
        end
      end

      private

      def render_login_shell
        div(class: 'relative flex items-center') do
          div(class: 'absolute -left-5 top-8 hidden h-32 w-32 rounded-full bg-[rgba(194,65,12,0.10)] blur-3xl lg:block')
          div(class: 'absolute -bottom-2 right-6 hidden h-24 w-24 rounded-full bg-[rgba(59,130,246,0.10)] blur-3xl lg:block')
          render RubyUI::Card.new(
            class: 'relative w-full overflow-hidden rounded-[2.5rem] border border-black/10 bg-[rgba(255,252,247,0.92)] shadow-[0_35px_90px_-38px_rgba(15,23,42,0.45)] backdrop-blur-xl'
          ) do
            div(class: 'lg:flex') do
              div(class: 'p-8 sm:p-10 lg:flex-1 lg:p-12') do
                render_login_header
                flash_section
                render_login_form
                render_passkey_section
                render_oauth_section
                render_signup_prompt
              end
              render_security_panel
            end
          end
        end
      end

      def render_login_header
        div(class: 'mb-10 space-y-6') do
          div(class: 'flex items-center justify-between gap-4') do
            div(class: 'inline-flex items-center gap-3 rounded-full border border-black/10 bg-white/70 px-4 py-2 text-[0.68rem] font-black uppercase tracking-[0.28em] text-zinc-700 shadow-sm') do
              span(class: 'inline-flex h-8 w-8 items-center justify-center rounded-full bg-zinc-950 text-white') do
                render Components::Icons::Pill.new(size: 16)
              end
              span { t('app.name') }
            end
            div(class: 'hidden text-right lg:block') do
              p(class: 'text-[0.68rem] font-black uppercase tracking-[0.3em] text-zinc-500') { 'Secure entry' }
              p(class: 'mt-1 text-sm text-zinc-600') { 'Passkeys, passwords, and provider login' }
            end
          end
          div(class: 'space-y-3') do
            Heading(level: 2, size: '8', class: 'font-black leading-[0.95] tracking-[-0.045em] text-zinc-950') { t('sessions.login.heading') }
            Text(size: '3', class: 'max-w-xl text-base leading-7 text-zinc-600') { t('sessions.login.subheading') }
          end
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
        div(class: 'space-y-3') do
          render RubyUI::FormFieldLabel.new(for: 'email',
                                            class: 'ml-1 text-[0.68rem] font-black uppercase tracking-[0.28em] text-zinc-500') do
            t('sessions.login.email_label')
          end
          render RubyUI::Input.new(**email_input_attrs,
                                   class: 'h-16 rounded-[1.75rem] border border-black/10 bg-white px-5 text-[15px] text-zinc-900 shadow-[inset_0_1px_0_rgba(255,255,255,0.9)] transition-all placeholder:text-zinc-400 focus:border-zinc-950 focus:ring-4 focus:ring-[rgba(24,24,27,0.08)]')
        end
      end

      def render_credentials_field
        div(class: 'space-y-3') do
          div(class: 'flex items-center justify-between gap-4') do
            render RubyUI::FormFieldLabel.new(for: 'password',
                                              class: 'ml-1 text-[0.68rem] font-black uppercase tracking-[0.28em] text-zinc-500') do
              t('sessions.login.password_label')
            end
            render RubyUI::Link.new(href: view_context.rodauth.reset_password_request_path, variant: :link, size: :sm,
                                    class: 'p-0 text-[0.72rem] font-black uppercase tracking-[0.18em] text-amber-700 h-auto hover:text-zinc-950 hover:underline') do
              t('sessions.login.forgot_password')
            end
          end
          render RubyUI::Input.new(**password_input_attrs,
                                   class: 'h-16 rounded-[1.75rem] border border-black/10 bg-white px-5 text-[15px] text-zinc-900 shadow-[inset_0_1px_0_rgba(255,255,255,0.9)] transition-all placeholder:text-zinc-400 focus:border-zinc-950 focus:ring-4 focus:ring-[rgba(24,24,27,0.08)]')
        end
      end

      def render_options_field
        div(class: 'flex items-center') do
          div(class: 'flex items-center gap-3 cursor-pointer group') do
            input(
              type: 'checkbox', name: 'remember', id: 'remember', value: 't',
              class: 'h-4 w-4 cursor-pointer rounded-md border-2 border-zinc-300 bg-white text-zinc-950 transition-all focus:ring-zinc-950'
            )
            label(for: 'remember',
                  class: 'cursor-pointer select-none text-sm font-semibold text-zinc-600 transition-colors group-hover:text-zinc-950') do
              t('sessions.login.remember_me')
            end
          end
        end
      end

      def render_submit_button
        div(class: 'pt-2') do
          render RubyUI::Button.new(type: :submit, variant: :primary,
                                    class: 'w-full rounded-[1.75rem] bg-zinc-950 py-8 text-base font-black text-white shadow-[0_22px_45px_-24px_rgba(24,24,27,0.75)] transition-transform duration-300 hover:-translate-y-0.5 hover:bg-zinc-900') do
            span(class: 'inline-flex items-center gap-3') do
              span { t('sessions.login.submit') }
              render Components::Icons::ChevronRight.new(size: 18)
            end
          end
        end
      end

      def render_oauth_section
        return unless oauth_enabled?

        div(class: 'mt-10 border-t border-black/10 pt-8') do
          provider_name = ENV.fetch('OIDC_PROVIDER_NAME', 'OIDC')
          form(action: view_context.rodauth.omniauth_request_path(:oidc), method: 'post', data: { turbo: 'false' }) do
            input(type: 'hidden', name: 'authenticity_token', value: view_context.form_authenticity_token)
            render RubyUI::Button.new(type: :submit, variant: :outline,
                                      class: 'w-full rounded-[1.75rem] border border-black/10 bg-white py-7 font-bold text-zinc-700 shadow-sm transition-all hover:border-zinc-950 hover:bg-zinc-50 hover:text-zinc-950') do
              render_oidc_icon
              span { t('sessions.login.oauth_continue', provider: provider_name) }
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
    end
  end
end
