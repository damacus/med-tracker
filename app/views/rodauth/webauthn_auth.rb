# frozen_string_literal: true

module Views
  module Rodauth
    class WebauthnAuth < Views::Base
      def view_template
        div(class: 'relative min-h-screen bg-gradient-to-br from-sky-50 via-white to-indigo-100 py-16 sm:py-20') do
          decorative_glow

          div(class: 'relative mx-auto flex w-full max-w-2xl flex-col items-center gap-8 px-4 sm:px-6 lg:px-8') do
            header_section
            form_section
          end
        end
      end

      private

      def header_section
        div(class: 'mx-auto max-w-xl text-center space-y-3') do
          h1(class: 'text-3xl font-bold tracking-tight text-slate-800 sm:text-4xl') do
            'Use your passkey'
          end
          p(class: 'text-lg text-slate-600') do
            'Confirm with your device biometric or security key.'
          end
        end
      end

      def decorative_glow
        div(class: 'pointer-events-none absolute inset-x-0 top-24 flex justify-center opacity-60') do
          div(class: 'h-64 w-64 rounded-full bg-sky-200 blur-3xl sm:h-80 sm:w-80')
        end
      end

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
          csrf_token_field
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

      def csrf_token_field
        input(type: 'hidden', name: 'authenticity_token', value: view_context.form_authenticity_token)
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

      def card_classes
        'w-full backdrop-blur bg-white/90 shadow-2xl border border-white/70 ring-1 ring-black/5 rounded-2xl'
      end

      def rodauth
        view_context.rodauth
      end
    end
  end
end
