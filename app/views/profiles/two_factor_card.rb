# frozen_string_literal: true

module Views
  module Profiles
    # rubocop:disable Metrics/ClassLength
    class TwoFactorCard < Views::Base
      include Phlex::Rails::Helpers::ButtonTo
      include Phlex::Rails::Helpers::Routes

      attr_reader :account, :passkeys, :recovery_codes_count

      def initialize(account:, passkeys:, totp_enabled:, recovery_codes_count:)
        super()
        @account = account
        @passkeys = passkeys
        @totp_enabled = totp_enabled
        @recovery_codes_count = recovery_codes_count
      end

      def otp_disable_path
        view_context.rodauth.otp_disable_path
      end

      def view_template
        Card do
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
        div(class: 'space-y-3 border-t border-border pt-4') do
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
        div(class: 'flex items-center justify-between rounded-lg border border-border bg-secondary-container/60 p-3') do
          div(class: 'flex items-center gap-2') do
            render Components::Icons::CheckCircle.new(size: 20, class: 'text-green-600')
            div do
              p(class: 'text-sm font-medium text-foreground') { 'Recovery codes generated' }
              p(class: 'text-xs text-on-surface-variant') { recovery_codes_status }
            end
          end
          div(class: 'flex gap-2') do
            render RubyUI::Link.new(
              variant: :outlined,
              size: :sm,
              href: '/recovery-codes',
              data: modal_link_data
            ) { 'View codes' }
            button_to(
              'Regenerate',
              '/recovery-codes',
              method: :post,
              class: 'inline-flex min-h-11 items-center justify-center rounded-shape-full border border-outline px-3 py-2 text-sm font-medium ring-offset-background transition-colors hover:bg-tertiary-container hover:text-on-tertiary-container focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50',
              data: { turbo_confirm: 'This will invalidate your existing recovery codes. Continue?' },
              form: modal_form_data
            )
          end
        end
      end

      def render_passkeys_section
        div(class: 'space-y-3 border-t border-border pt-4') do
          render_section_header(
            'Passkeys',
            'Passwordless authentication using biometrics or security keys'
          )
          render_passkeys_list
        end
      end

      def render_passkeys_list
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
        div(class: 'flex items-center justify-between rounded-lg border border-border bg-card/70 p-3') do
          div(class: 'flex items-center gap-3 flex-1') do
            render Components::Icons::Passkey.new(size: 20, class: 'text-on-surface-variant')
            div do
              p(class: 'text-sm font-medium text-foreground') { passkey.nickname }
              p(class: 'text-xs text-on-surface-variant') do
                passkey_added_on(passkey)
              end
            end
          end
          render RubyUI::Link.new(
            href: passkey_remove_path(passkey),
            variant: :link,
            class: 'text-sm text-destructive hover:text-destructive/80 font-medium p-0 h-auto',
            data: modal_link_data
          ) { 'Remove' }
        end
      end

      def render_add_passkey_button
        div(class: 'pt-2') do
          render RubyUI::Link.new(
            variant: :outlined,
            size: :sm,
            href: '/webauthn-setup',
            data: modal_link_data
          ) { 'Add a passkey' }
        end
      end

      def render_section_header(title, description)
        div(class: 'space-y-1') do
          h3(class: 'text-sm font-semibold text-foreground') { title }
          p(class: 'text-xs text-on-surface-variant') { description }
        end
      end

      def render_enabled_method(status_text, disable_path:, disable_text:)
        div(class: 'flex items-center justify-between rounded-lg border border-success/40 bg-success-light p-3') do
          div(class: 'flex items-center gap-2') do
            render Components::Icons::CheckCircle.new(size: 20, class: 'text-green-600')
            p(class: 'text-sm font-medium text-success-text') { status_text }
          end
          render RubyUI::Link.new(
            variant: :outlined,
            size: :sm,
            href: disable_path,
            data: modal_link_data
          ) { disable_text }
        end
      end

      def render_disabled_method(status_text, setup_path:, setup_text:)
        div(class: 'flex items-center justify-between rounded-lg border border-border bg-secondary-container/60 p-3') do
          div(class: 'flex items-center gap-2') do
            render Components::Icons::XCircle.new(size: 20, class: 'text-on-surface-variant')
            p(class: 'text-sm text-on-surface-variant') { status_text }
          end
          render RubyUI::Link.new(
            variant: :outlined,
            size: :sm,
            href: setup_path,
            data: modal_link_data
          ) { setup_text }
        end
      end

      def totp_enabled?
        @totp_enabled
      end

      def recovery_codes_exist?
        recovery_codes_count.positive?
      end

      def recovery_codes_status
        "#{recovery_codes_count} codes available"
      end

      def passkey_added_on(passkey)
        "Added #{passkey.created_at.strftime('%B %d, %Y')}"
      end

      def passkey_remove_path(passkey)
        query = URI.encode_www_form(view_context.rodauth.webauthn_remove_param => passkey.webauthn_id)
        "/webauthn-remove?#{query}"
      end

      def modal_link_data
        { turbo_frame: 'modal' }
      end

      def modal_form_data
        { data: modal_link_data }
      end
    end
    # rubocop:enable Metrics/ClassLength
  end
end
