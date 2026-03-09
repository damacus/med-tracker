# frozen_string_literal: true

module Views
  module Rodauth
    module LoginPasskeySupport
      include Views::Rodauth::LoginPasskeyUiSupport

      private

      def render_passkey_section
        credential_options = rodauth.webauthn_credential_options_for_get

        div(**passkey_section_attributes) do
          passkey_section_header
          passkey_trigger_button
          passkey_error_message
          render_passkey_login_form(credential_options)
        end
      rescue StandardError => e
        Rails.logger.error("Failed to render passkey section: #{e.message}")
        nil
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
    end
  end
end
