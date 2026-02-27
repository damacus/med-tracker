# frozen_string_literal: true

module Views
  module Profiles
    # rubocop:disable Metrics/ClassLength
    class TwoFactorCard < Views::Base
      include Phlex::Rails::Helpers::ButtonTo
      include Phlex::Rails::Helpers::Routes

      attr_reader :account

      def initialize(account:)
        super()
        @account = account
      end

      def otp_disable_path
        view_context.rodauth.otp_disable_path
      end

      def view_template
        render Card.new do
          render CardHeader.new do
            render(CardTitle.new { 'Two-Factor Authentication' })
            render(CardDescription.new do
              'Secure your account with multiple authentication methods'
            end)
          end
          render CardContent.new(class: 'space-y-6') do
            render_totp_section
            render_recovery_codes_section
            render_passkeys_section
          end
        end
      end

      private

      def render_totp_section
        div(class: 'space-y-3') do
          render_section_header(
            'Authenticator App (TOTP)',
            'Use an app like Google Authenticator or 1Password to generate codes'
          )
          render_totp_status
        end
      end

      def render_totp_status
        if totp_enabled?
          render_enabled_method(
            'Authenticator app is active',
            disable_path: otp_disable_path,
            disable_text: 'Disable'
          )
        else
          render_disabled_method(
            'Not configured',
            setup_path: '/otp-setup',
            setup_text: 'Set up authenticator app'
          )
        end
      end

      def render_recovery_codes_section
        div(class: 'space-y-3 pt-4 border-t border-slate-200') do
          render_section_header(
            'Recovery Codes',
            'Use these codes to access your account if you lose your 2FA device'
          )
          render_recovery_codes_status
        end
      end

      def render_recovery_codes_status
        if recovery_codes_exist?
          render_recovery_codes_actions
        else
          render_disabled_method(
            'Not generated',
            setup_path: '/recovery-codes',
            setup_text: 'Generate recovery codes'
          )
        end
      end

      def render_recovery_codes_actions
        div(class: 'flex items-center justify-between p-3 border border-slate-200 rounded-lg bg-slate-50') do
          div(class: 'flex items-center gap-2') do
            render Components::Icons::CheckCircle.new(size: 20, class: 'text-green-600')
            div do
              p(class: 'text-sm font-medium text-slate-900') { 'Recovery codes generated' }
              p(class: 'text-xs text-slate-600') { "#{recovery_codes_count} codes available" }
            end
          end
          div(class: 'flex gap-2') do
            render RubyUI::Link.new(
              variant: :outline,
              size: :sm,
              href: '/recovery-codes'
            ) { 'View codes' }
            button_to(
              'Regenerate',
              '/recovery-codes',
              method: :post,
              class: 'inline-flex items-center justify-center rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:opacity-50 disabled:pointer-events-none ring-offset-background border border-input hover:bg-accent hover:text-accent-foreground h-9 px-3',
              data: { turbo_confirm: 'This will invalidate your existing recovery codes. Continue?' }
            )
          end
        end
      end

      def render_passkeys_section
        div(class: 'space-y-3 pt-4 border-t border-slate-200') do
          render_section_header(
            'Passkeys',
            'Passwordless authentication using biometrics or security keys'
          )
          render_passkeys_list
        end
      end

      def render_passkeys_list
        passkeys = account.account_webauthn_keys.order(created_at: :desc)

        if passkeys.empty?
          render_empty_passkeys_state
        else
          div(class: 'space-y-3') do
            passkeys.each do |passkey|
              render_passkey_item(passkey)
            end
            render_add_passkey_button
          end
        end
      end

      def render_empty_passkeys_state
        render_disabled_method(
          'No passkeys registered',
          setup_path: '/webauthn-setup',
          setup_text: 'Add a passkey'
        )
      end

      def render_passkey_item(passkey)
        div(class: 'flex items-center justify-between p-3 border border-slate-200 rounded-lg') do
          div(class: 'flex items-center gap-3 flex-1') do
            render Components::Icons::Key.new(size: 20, class: 'text-slate-600')
            div do
              p(class: 'text-sm font-medium text-slate-900') { passkey.nickname }
              p(class: 'text-xs text-slate-500') do
                "Added #{passkey.created_at.strftime('%B %d, %Y')}"
              end
            end
          end
          render RubyUI::Link.new(
            href: "/webauthn-remove?#{URI.encode_www_form(view_context.rodauth.webauthn_remove_param => passkey.webauthn_id)}",
            variant: :link,
            class: 'text-sm text-destructive hover:text-destructive/80 font-medium p-0 h-auto'
          ) { 'Remove' }
        end
      end

      def render_add_passkey_button
        div(class: 'pt-2') do
          render RubyUI::Link.new(
            variant: :outline,
            size: :sm,
            href: '/webauthn-setup'
          ) { 'Add a passkey' }
        end
      end

      def render_section_header(title, description)
        div(class: 'space-y-1') do
          h3(class: 'text-sm font-semibold text-slate-900') { title }
          p(class: 'text-xs text-slate-600') { description }
        end
      end

      def render_enabled_method(status_text, disable_path:, disable_text:)
        div(class: 'flex items-center justify-between p-3 border border-slate-200 rounded-lg bg-green-50 border-green-200') do
          div(class: 'flex items-center gap-2') do
            render Components::Icons::CheckCircle.new(size: 20, class: 'text-green-600')
            p(class: 'text-sm font-medium text-slate-900') { status_text }
          end
          render RubyUI::Link.new(
            variant: :outline,
            size: :sm,
            href: disable_path
          ) { disable_text }
        end
      end

      def render_disabled_method(status_text, setup_path:, setup_text:)
        div(class: 'flex items-center justify-between p-3 border border-slate-200 rounded-lg bg-slate-50') do
          div(class: 'flex items-center gap-2') do
            render Components::Icons::XCircle.new(size: 20, class: 'text-slate-400')
            p(class: 'text-sm text-slate-600') { status_text }
          end
          render RubyUI::Link.new(
            variant: :default,
            size: :sm,
            href: setup_path
          ) { setup_text }
        end
      end

      def totp_enabled?
        return false unless ActiveRecord::Base.connection.table_exists?('account_otp_keys')

        AccountOtpKey.exists?(id: account.id)
      rescue StandardError
        false
      end

      def recovery_codes_exist?
        return false unless ActiveRecord::Base.connection.table_exists?('account_recovery_codes')

        recovery_codes_count.positive?
      rescue StandardError
        false
      end

      def recovery_codes_count
        return @recovery_codes_count if defined?(@recovery_codes_count)

        @recovery_codes_count = if ActiveRecord::Base.connection.table_exists?('account_recovery_codes')
                                  AccountRecoveryCode.where(id: account.id).count
                                else
                                  0
                                end
      rescue StandardError
        @recovery_codes_count = 0
      end
    end
    # rubocop:enable Metrics/ClassLength
  end
end
