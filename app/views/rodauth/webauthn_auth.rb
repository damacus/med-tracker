# frozen_string_literal: true

module Views
  module Rodauth
    class WebauthnAuth < Views::Rodauth::Base
      def view_template
        page_layout do
          render_page_header(
            title: 'Use your passkey',
            subtitle: 'Confirm with your device biometric or security key.'
          )
          form_section
        end
      end

      private

      def form_section
        render RubyUI::Card.new(class: card_classes) do
          render RubyUI::CardHeader.new(class: 'space-y-2 bg-white/60') do
            render RubyUI::CardTitle.new(class: 'text-xl font-semibold text-slate-900') do
              'Passkey authentication'
            end
            render RubyUI::CardDescription.new(class: 'text-base text-slate-600') do
              'Tap continue to use your passkey for verification.'
            end
          end
          render RubyUI::CardContent.new(class: 'space-y-6 p-6 sm:p-8') do
            webauthn_form
          end
        end
      end

      def webauthn_form
        credential_options = rodauth.webauthn_credential_options_for_get
        render_webauthn_form(credential_options)
        render_webauthn_script
      end

      def render_webauthn_form(credential_options)
        form(**webauthn_form_attributes(credential_options)) do
          additional_tags = rodauth.webauthn_auth_additional_form_tags
          safe(additional_tags) if additional_tags.present?
          authenticity_token_field
          challenge_fields(credential_options)
          hidden_credential_input
          submit_button
        end
      end

      def webauthn_form_attributes(credential_options)
        {
          method: :post,
          action: rodauth.webauthn_auth_form_path,
          role: 'form',
          id: 'webauthn-auth-form',
          data_credential_options: credential_options.as_json.to_json,
          class: 'space-y-6'
        }
      end

      def render_webauthn_script
        script(src: "#{rodauth.webauthn_js_host}#{rodauth.webauthn_auth_js_path}")
      end

      def challenge_fields(credential_options)
        input(type: 'hidden', name: rodauth.webauthn_auth_challenge_param, value: credential_options.challenge)
        input(
          type: 'hidden',
          name: rodauth.webauthn_auth_challenge_hmac_param,
          value: rodauth.compute_hmac(credential_options.challenge)
        )
      end

      def hidden_credential_input
        input(
          type: 'text',
          name: rodauth.webauthn_auth_param,
          id: 'webauthn-auth',
          value: '',
          class: 'sr-only',
          aria_hidden: 'true'
        )
      end

      def submit_button
        render RubyUI::Button.new(type: :submit, variant: :primary, size: :md, class: 'w-full') do
          rodauth.webauthn_auth_button
        end
      end
    end
  end
end
