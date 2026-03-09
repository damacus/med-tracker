# frozen_string_literal: true

module Views
  module Rodauth
    module LoginPasskeySupport
      private

      def render_passkey_section
        credential_options = rodauth.webauthn_credential_options_for_get

        div(**passkey_section_attributes) do
          passkey_section_header
          passkey_trigger_button
          passkey_error_message
          render_passkey_login_form(credential_options)
          render_passkey_boot_script
        end
      end

      def passkey_section_attributes
        {
          id: 'passkey-login-section',
          hidden: true,
          class: 'mt-10 border-t border-border pt-10'
        }
      end

      def passkey_section_header
        div(class: 'mb-5 space-y-1') do
          p(class: 'text-[10px] font-black uppercase tracking-widest text-muted-foreground') do
            t('sessions.login.passkey_label')
          end
          p(class: 'text-sm text-muted-foreground') do
            t('sessions.login.passkey_helper')
          end
        end
      end

      def passkey_trigger_button
        button(**passkey_trigger_attributes) do
          span(class: 'inline-flex items-center justify-center gap-3') { t('sessions.login.passkey_cta') }
        end
      end

      def passkey_trigger_attributes
        {
          type: 'button',
          id: 'passkey-login-trigger',
          hidden: true,
          disabled: true,
          class: 'w-full rounded-2xl border border-border bg-background/80 px-5 py-5 text-sm font-bold text-foreground shadow-sm transition-all hover:bg-accent disabled:cursor-not-allowed disabled:opacity-60',
          data_error_unsupported: t('sessions.login.passkey_not_supported'),
          data_error_failed: t('sessions.login.passkey_error')
        }
      end

      def passkey_error_message
        p(
          id: 'passkey-login-error',
          hidden: true,
          role: 'status',
          aria_live: 'polite',
          class: 'mt-3 text-sm font-medium text-destructive'
        )
      end

      def render_passkey_login_form(credential_options)
        form(**passkey_form_attributes(credential_options)) do
          additional_tags = rodauth.webauthn_auth_additional_form_tags
          safe(additional_tags) if additional_tags.present?
          authenticity_token_field
          passkey_challenge_fields(credential_options)
          passkey_auth_field
        end
      end

      def passkey_form_attributes(credential_options)
        {
          method: :post,
          action: rodauth.webauthn_login_path,
          role: 'form',
          id: 'webauthn-login-form',
          hidden: true,
          data_credential_options: credential_options.as_json.to_json
        }
      end

      def passkey_challenge_fields(credential_options)
        input(type: 'hidden', name: rodauth.webauthn_auth_challenge_param, value: credential_options.challenge)
        input(
          type: 'hidden',
          name: rodauth.webauthn_auth_challenge_hmac_param,
          value: rodauth.compute_hmac(credential_options.challenge)
        )
      end

      def passkey_auth_field
        input(
          type: 'text',
          name: rodauth.webauthn_auth_param,
          id: 'webauthn-auth',
          value: '',
          class: 'sr-only',
          aria_hidden: 'true'
        )
      end

      def render_passkey_boot_script
        script(nonce: view_context.content_security_policy_nonce) do
          safe('window.MedTrackerAuth && window.MedTrackerAuth.initPasskeyLogin();')
        end
      end
    end
  end
end
