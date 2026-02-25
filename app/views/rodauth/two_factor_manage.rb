# frozen_string_literal: true

module Views
  module Rodauth
    class TwoFactorManage < Views::Rodauth::Base
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        page_layout do
          render_page_header(title: header_title, subtitle: header_subtitle)
          content_section
        end
      end

      private

      def header_title
        if rodauth.two_factor_authentication_setup?
          'Manage Two-Factor Authentication'
        else
          'Set Up Two-Factor Authentication'
        end
      end

      def header_subtitle
        if rodauth.two_factor_authentication_setup?
          'Manage your authentication methods or add additional security.'
        else
          'Choose a method to secure your account with two-factor authentication.'
        end
      end

      def content_section
        render RubyUI::Card.new(class: card_classes) do
          render_card_header
          render_card_content
        end
      end

      def card_classes
        "#{CARD_CLASSES} overflow-hidden"
      end

      def render_card_header
        render RubyUI::CardHeader.new(class: 'space-y-2 bg-white/60') do
          render RubyUI::CardTitle.new(class: 'text-xl font-semibold text-slate-900') do
            'Authentication Methods'
          end
          render RubyUI::CardDescription.new(class: 'text-base text-slate-600') do
            plain 'Choose how you want to verify your identity when signing in.'
          end
        end
      end

      def render_card_content
        render RubyUI::CardContent.new(class: 'space-y-4 p-6 sm:p-8') do
          render_webauthn_option
          render_totp_option
          render_recovery_codes_option if rodauth.two_factor_authentication_setup?
        end
      end

      def render_webauthn_option
        render_auth_method_card({
                                  title: 'Passkeys',
                                  description: 'Use biometrics or a security key for passwordless login.',
                                  icon: :passkey,
                                  setup_path: rodauth.webauthn_setup_path,
                                  setup_text: 'Set up passkey',
                                  enabled: webauthn_enabled?,
                                  manage_path: webauthn_enabled? ? rodauth.webauthn_remove_path : nil,
                                  manage_text: 'Manage passkeys'
                                })
      end

      def render_totp_option
        render_auth_method_card({
                                  title: 'Authenticator App (TOTP)',
                                  description: 'Use an app like Google Authenticator or 1Password.',
                                  icon: :totp,
                                  setup_path: rodauth.otp_setup_path,
                                  setup_text: 'Set up authenticator',
                                  enabled: totp_enabled?,
                                  manage_path: totp_enabled? ? rodauth.otp_disable_path : nil,
                                  manage_text: 'Disable TOTP'
                                })
      end

      def render_recovery_codes_option
        render_auth_method_card({
                                  title: 'Recovery Codes',
                                  description: 'Backup codes for when you lose access to your device.',
                                  icon: :recovery,
                                  setup_path: rodauth.recovery_codes_path,
                                  setup_text: 'View recovery codes',
                                  enabled: recovery_codes_enabled?,
                                  manage_path: nil,
                                  manage_text: nil
                                })
      end

      def render_auth_method_card(method_config) # rubocop:disable Metrics/AbcSize
        div(class: 'flex items-start gap-4 p-4 rounded-xl border border-slate-200 bg-white hover:bg-slate-50 transition-colors') do
          render_method_icon(method_config[:icon], method_config[:enabled])

          div(class: 'flex-1 min-w-0') do
            div(class: 'flex items-center gap-2') do
              h4(class: 'font-medium text-slate-900') { method_config[:title] }
              render_status_badge(method_config[:enabled]) if method_config[:enabled]
            end
            p(class: 'text-sm text-slate-600 mt-1') { method_config[:description] }
          end

          div(class: 'flex-shrink-0') do
            if method_config[:enabled] && method_config[:manage_path]
              render RubyUI::Link.new(href: method_config[:manage_path], variant: :outline, size: :sm) { method_config[:manage_text] }
            else
              render RubyUI::Link.new(href: method_config[:setup_path], variant: :primary, size: :sm) { method_config[:setup_text] }
            end
          end
        end
      end

      def render_method_icon(icon_type, enabled)
        bg_class = enabled ? 'bg-green-100' : 'bg-slate-100'
        icon_class = enabled ? 'text-green-600' : 'text-slate-500'

        div(class: "flex-shrink-0 w-12 h-12 #{bg_class} rounded-full flex items-center justify-center") do
          case icon_type
          when :passkey
            render_passkey_icon(icon_class)
          when :totp
            render_totp_icon(icon_class)
          when :recovery
            render_recovery_icon(icon_class)
          end
        end
      end

      def render_passkey_icon(icon_class)
        render Icons::Fingerprint.new(size: 24, class: icon_class)
      end

      def render_totp_icon(icon_class)
        render Icons::Smartphone.new(size: 24, class: icon_class)
      end

      def render_recovery_icon(icon_class)
        render Icons::Key.new(size: 24, class: icon_class)
      end

      def render_status_badge(enabled)
        return unless enabled

        span(class: 'inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800') do
          'Enabled'
        end
      end

      def webauthn_enabled?
        rodauth.uses_webauthn_authentication?
      rescue StandardError
        false
      end

      def totp_enabled?
        rodauth.otp_exists?
      rescue StandardError
        false
      end

      def recovery_codes_enabled?
        rodauth.recovery_codes_exist?
      rescue StandardError
        false
      end
    end
  end
end
