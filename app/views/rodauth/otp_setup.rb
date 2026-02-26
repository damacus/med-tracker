# frozen_string_literal: true

module Views
  module Rodauth
    class OtpSetup < Views::Rodauth::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        page_layout do
          render_page_header(
            title: t('rodauth.views.otp_setup.page_title'),
            subtitle: t('rodauth.views.otp_setup.page_subtitle')
          )
          form_section
        end
      end

      private

      def form_section
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
            t('rodauth.views.otp_setup.card_title')
          end
          render RubyUI::CardDescription.new(class: 'text-base text-slate-600') do
            plain t('rodauth.views.otp_setup.card_description')
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
            div(class: 'w-48 h-48 flex items-center justify-center [&_img]:max-w-full [&_img]:max-h-full [&_svg]:max-w-full [&_svg]:max-h-full') do
              qr_html = view_context.rodauth.otp_qr_code
              safe(qr_html)
            end
          end
          p(class: 'text-sm text-slate-500 text-center') do
            t('rodauth.views.otp_setup.qr_hint')
          end
        end
      end

      def render_manual_entry_section
        details(class: 'border-t border-slate-200 pt-6 group') do
          summary(class: 'flex items-center gap-2 text-sm text-slate-600 hover:text-slate-900 transition-colors cursor-pointer list-none') do
            chevron_icon
            span { t('rodauth.views.otp_setup.manual_entry_summary') }
          end

          div(class: 'mt-4 space-y-3') do
            render_secret_field
            render_provisioning_url
          end
        end
      end

      def chevron_icon
        render Icons::ChevronRight.new(size: 16, class: 'transition-transform group-open:rotate-90')
      end

      def render_secret_field
        div(class: 'bg-slate-50 rounded-lg p-4 space-y-2') do
          label(class: 'text-xs font-medium text-slate-500 uppercase tracking-wide') { t('rodauth.views.otp_setup.secret_key') }
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
          label(class: 'text-xs font-medium text-slate-500 uppercase tracking-wide') { t('rodauth.views.otp_setup.provisioning_url') }
          code(class: 'block font-mono text-xs text-slate-600 break-all select-all') do
            plain view_context.rodauth.otp_provisioning_uri
          end
        end
      end

      def copy_button(text)
        button(
          type: 'button',
          class: 'p-2 text-slate-400 hover:text-slate-600 transition-colors',
          title: t('rodauth.views.otp_setup.copy_to_clipboard'),
          data: { action: 'click->clipboard#copy', clipboard_text_param: text }
        ) do
          render Icons::Copy.new(size: 16)
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
            otp_secret_field
            password_field
            otp_code_field
            submit_button
          end
        end
      end

      def otp_secret_field
        rodauth = view_context.rodauth
        input(type: 'hidden', name: rodauth.otp_setup_param, value: rodauth.otp_user_key)
        input(type: 'hidden', name: rodauth.otp_setup_raw_param, value: rodauth.otp_key) if rodauth.otp_keys_use_hmac?
      end

      def password_field
        render RubyUI::FormField.new do
          render RubyUI::FormFieldLabel.new(for: 'password') { t('rodauth.views.otp_setup.current_password_label') }
          render RubyUI::Input.new(
            type: :password,
            name: 'password',
            id: 'password',
            required: true,
            autocomplete: 'current-password',
            placeholder: t('rodauth.views.otp_setup.current_password_placeholder')
          )
          p(class: 'text-xs text-slate-500 mt-1') { t('rodauth.views.otp_setup.current_password_hint') }
        end
      end

      def otp_code_field
        render RubyUI::FormField.new do
          render RubyUI::FormFieldLabel.new(for: 'otp') { t('rodauth.views.otp_setup.auth_code_label') }
          render RubyUI::Input.new(
            type: :text,
            name: view_context.rodauth.otp_auth_param,
            id: 'otp',
            required: true,
            autocomplete: 'one-time-code',
            inputmode: 'numeric',
            pattern: '[0-9]*',
            maxlength: 6,
            placeholder: t('rodauth.views.otp_setup.auth_code_placeholder')
          )
          p(class: 'text-xs text-slate-500 mt-1') { t('rodauth.views.otp_setup.auth_code_hint') }
        end
      end

      def submit_button
        render RubyUI::Button.new(type: :submit, variant: :primary, size: :md, class: 'w-full') do
          t('rodauth.views.otp_setup.submit')
        end
      end
    end
  end
end
