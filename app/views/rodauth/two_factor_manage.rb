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
        render RubyUI::Card.new(class: "#{CARD_CLASSES} overflow-hidden") do
          render_card_header
          render_card_content
        end
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
        render Views::Rodauth::AuthMethodCard.new(
          title: 'Passkeys',
          description: 'Use biometrics or a security key for passwordless login.',
          icon: :passkey,
          setup_path: rodauth.webauthn_setup_path,
          setup_text: 'Set up passkey',
          enabled: webauthn_enabled?,
          manage_path: webauthn_enabled? ? rodauth.webauthn_remove_path : nil,
          manage_text: 'Manage passkeys'
        )
      end

      def render_totp_option
        render Views::Rodauth::AuthMethodCard.new(
          title: 'Authenticator App (TOTP)',
          description: 'Use an app like Google Authenticator or 1Password.',
          icon: :totp,
          setup_path: rodauth.otp_setup_path,
          setup_text: 'Set up authenticator',
          enabled: totp_enabled?,
          manage_path: totp_enabled? ? rodauth.otp_disable_path : nil,
          manage_text: 'Disable TOTP'
        )
      end

      def render_recovery_codes_option
        render Views::Rodauth::AuthMethodCard.new(
          title: 'Recovery Codes',
          description: 'Backup codes for when you lose access to your device.',
          icon: :recovery,
          setup_path: rodauth.recovery_codes_path,
          setup_text: 'View recovery codes',
          enabled: recovery_codes_enabled?
        )
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
