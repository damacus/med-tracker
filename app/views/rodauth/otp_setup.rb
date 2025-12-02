# frozen_string_literal: true

module Views
  module Rodauth
    class OtpSetup < Views::Base
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
            'Set Up Two-Factor Authentication'
          end
          p(class: 'text-lg text-slate-600') do
            'Scan the QR code with your authenticator app to enable 2FA.'
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
            'Authenticator Setup'
          end
          render RubyUI::CardDescription.new(class: 'text-base text-slate-600') do
            plain 'Use an authenticator app like Google Authenticator, Authy, or 1Password to scan the QR code below.'
          end
        end
      end

      def render_card_content
        render RubyUI::CardContent.new(class: 'space-y-6 p-6 sm:p-8') do
          render_qr_code_section
          render_manual_entry_section
          render_setup_form
        end
      end

      def render_qr_code_section
        div(class: 'flex flex-col items-center space-y-4') do
          div(class: 'p-4 bg-white rounded-xl shadow-sm border border-slate-200') do
            # The QR code is rendered by Rodauth - we need to constrain its size
            div(class: 'w-48 h-48 flex items-center justify-center [&_img]:max-w-full [&_img]:max-h-full [&_svg]:max-w-full [&_svg]:max-h-full') do
              # Use safe to render the QR code HTML from Rodauth
              qr_html = view_context.rodauth.otp_qr_code
              safe(qr_html)
            end
          end
          p(class: 'text-sm text-slate-500 text-center') do
            'Scan this QR code with your authenticator app'
          end
        end
      end

      def render_manual_entry_section
        details(class: 'border-t border-slate-200 pt-6 group') do
          summary(class: 'flex items-center gap-2 text-sm text-slate-600 hover:text-slate-900 transition-colors cursor-pointer list-none') do
            chevron_icon
            span { "Can't scan? Enter the code manually" }
          end

          div(class: 'mt-4 space-y-3') do
            render_secret_field
            render_provisioning_url
          end
        end
      end

      def chevron_icon
        svg(
          class: 'w-4 h-4 transition-transform group-open:rotate-90',
          fill: 'none',
          viewBox: '0 0 24 24',
          stroke: 'currentColor',
          stroke_width: '2'
        ) do |s|
          s.path(stroke_linecap: 'round', stroke_linejoin: 'round', d: 'M9 5l7 7-7 7')
        end
      end

      def render_secret_field
        div(class: 'bg-slate-50 rounded-lg p-4 space-y-2') do
          label(class: 'text-xs font-medium text-slate-500 uppercase tracking-wide') { 'Secret Key' }
          div(class: 'flex items-center gap-2') do
            code(class: 'flex-1 font-mono text-sm text-slate-800 break-all select-all') do
              plain view_context.rodauth.otp_user_key
            end
            copy_button(view_context.rodauth.otp_user_key)
          end
        end
      end

      def render_provisioning_url
        div(class: 'bg-slate-50 rounded-lg p-4 space-y-2') do
          label(class: 'text-xs font-medium text-slate-500 uppercase tracking-wide') { 'Provisioning URL' }
          code(class: 'block font-mono text-xs text-slate-600 break-all select-all') do
            plain view_context.rodauth.otp_provisioning_uri
          end
        end
      end

      def copy_button(text)
        button(
          type: 'button',
          class: 'p-2 text-slate-400 hover:text-slate-600 transition-colors',
          title: 'Copy to clipboard',
          data: { action: 'click->clipboard#copy', clipboard_text_param: text }
        ) do
          svg(class: 'w-4 h-4', fill: 'none', viewBox: '0 0 24 24', stroke: 'currentColor', stroke_width: '2') do |s|
            s.path(
              stroke_linecap: 'round',
              stroke_linejoin: 'round',
              d: 'M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z'
            )
          end
        end
      end

      def render_setup_form
        div(class: 'border-t border-slate-200 pt-6') do
          render RubyUI::Form.new(
            action: view_context.rodauth.otp_setup_path,
            method: :post,
            class: 'space-y-6',
            data_turbo: 'false'
          ) do
            authenticity_token_field
            password_field
            otp_code_field
            submit_button
          end
        end
      end

      def authenticity_token_field
        input(type: 'hidden', name: 'authenticity_token', value: view_context.form_authenticity_token)
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
          p(class: 'text-xs text-slate-500 mt-1') { 'Required to verify your identity' }
        end
      end

      def otp_code_field
        render RubyUI::FormField.new do
          render RubyUI::FormFieldLabel.new(for: 'otp') { 'Authentication Code' }
          render RubyUI::Input.new(
            type: :text,
            name: view_context.rodauth.otp_auth_param,
            id: 'otp',
            required: true,
            autocomplete: 'one-time-code',
            inputmode: 'numeric',
            pattern: '[0-9]*',
            maxlength: 6,
            placeholder: 'Enter 6-digit code from app'
          )
          p(class: 'text-xs text-slate-500 mt-1') { 'Enter the code shown in your authenticator app' }
        end
      end

      def submit_button
        render RubyUI::Button.new(type: :submit, variant: :primary, size: :md, class: 'w-full') do
          'Enable Two-Factor Authentication'
        end
      end
    end
  end
end
