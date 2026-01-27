# frozen_string_literal: true

module Views
  module Rodauth
    class WebauthnSetup < Views::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::LinkTo

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
            'Set Up Passkey Authentication'
          end
          p(class: 'text-lg text-slate-600') do
            'Add a passkey for fast, secure passwordless login.'
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
          render_card_header
          render_card_content
        end
      end

      def card_classes
        'w-full backdrop-blur bg-white/90 shadow-2xl border border-white/70 ' \
          'ring-1 ring-black/5 rounded-2xl overflow-hidden'
      end

      def render_card_header
        render RubyUI::CardHeader.new(class: 'space-y-2 bg-white/60') do
          render RubyUI::CardTitle.new(class: 'text-xl font-semibold text-slate-900') do
            'Register a Passkey'
          end
          render RubyUI::CardDescription.new(class: 'text-base text-slate-600') do
            plain 'Passkeys use your device\'s biometrics or security key for secure, passwordless authentication.'
          end
        end
      end

      def render_card_content
        render RubyUI::CardContent.new(class: 'space-y-6 p-6 sm:p-8') do
          render_info_section
          render_setup_form
        end
      end

      def render_info_section
        div(class: 'bg-sky-50 rounded-xl p-4 space-y-3') do
          div(class: 'flex items-start gap-3') do
            render_info_icon
            div(class: 'space-y-1') do
              h4(class: 'font-medium text-slate-900') { 'What are passkeys?' }
              p(class: 'text-sm text-slate-600') do
                'Passkeys are a secure replacement for passwords. They use your device\'s built-in security ' \
                  '(like Face ID, Touch ID, or Windows Hello) to verify your identity.'
              end
            end
          end
        end
      end

      def render_info_icon
        div(class: 'flex-shrink-0 w-10 h-10 bg-sky-100 rounded-full flex items-center justify-center') do
          svg(class: 'w-5 h-5 text-sky-600', fill: 'none', viewBox: '0 0 24 24', stroke: 'currentColor', stroke_width: '2') do |s|
            s.path(stroke_linecap: 'round', stroke_linejoin: 'round', d: 'M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z')
          end
        end
      end

      def render_setup_form
        div(class: 'border-t border-slate-200 pt-6') do
          form(
            method: :post,
            action: view_context.rodauth.webauthn_setup_path,
            id: 'webauthn-setup-form',
            class: 'space-y-6'
          ) do
            additional_tags = view_context.rodauth.webauthn_setup_additional_form_tags
            safe(additional_tags) if additional_tags.present?
            authenticity_token_field
            hidden_webauthn_fields
            password_field
            submit_button
          end

          render_webauthn_script
        end
      end

      def authenticity_token_field
        input(type: 'hidden', name: 'authenticity_token', value: view_context.form_authenticity_token)
      end

      def hidden_webauthn_fields
        input(type: 'hidden', id: 'webauthn-setup', name: view_context.rodauth.webauthn_setup_param, value: '')
      end

      def password_field
        render RubyUI::FormField.new do
          render RubyUI::FormFieldLabel.new(for: 'password') { 'Current Password' }
          render RubyUI::Input.new(
            type: :password,
            name: 'password',
            id: 'password',
            required: true,
            autocomplete: 'current-password',
            placeholder: 'Enter your password to confirm'
          )
          p(class: 'text-xs text-slate-500 mt-1') { 'Required to verify your identity before adding a passkey' }
        end
      end

      def submit_button
        render RubyUI::Button.new(type: :submit, variant: :primary, size: :md, class: 'w-full') do
          'Register Passkey'
        end
      end

      def render_webauthn_script
        script(src: "#{view_context.rodauth.webauthn_js_host}#{view_context.rodauth.webauthn_setup_js_path}")
      end
    end
  end
end
