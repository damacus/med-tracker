# frozen_string_literal: true

module Views
  module Rodauth
    class OtpAuth < Views::Base
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
            'Enter your authentication code'
          end
          p(class: 'text-lg text-slate-600') do
            'Use the code from your authenticator app to continue.'
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
              'Authenticator code'
            end
            render RubyUI::CardDescription.new(class: 'text-base text-slate-600') do
              'Enter the 6-digit code to finish signing in.'
            end
          end
          render RubyUI::CardContent.new(class: 'space-y-6 p-6 sm:p-8') do
            otp_form
          end
        end
      end

      def otp_form
        render RubyUI::Form.new(action: rodauth.otp_auth_path, method: :post, class: 'space-y-6', data_turbo: 'false') do
          additional_tags = rodauth.otp_auth_additional_form_tags
          safe(additional_tags) if additional_tags.present?
          authenticity_token_field
          otp_code_field
          submit_button
        end
      end

      def authenticity_token_field
        input(type: 'hidden', name: 'authenticity_token', value: view_context.form_authenticity_token)
      end

      def otp_code_field
        render RubyUI::FormField.new do
          render RubyUI::FormFieldLabel.new(for: 'otp-auth-code') { rodauth.otp_auth_label }
          render RubyUI::Input.new(
            type: :text,
            name: rodauth.otp_auth_param,
            id: 'otp-auth-code',
            required: true,
            autocomplete: 'one-time-code',
            inputmode: 'numeric',
            pattern: '[0-9]*',
            maxlength: 6,
            placeholder: 'Enter 6-digit code'
          )
        end
      end

      def submit_button
        render RubyUI::Button.new(type: :submit, variant: :primary, size: :md, class: 'w-full') do
          rodauth.otp_auth_button
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
