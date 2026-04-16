# frozen_string_literal: true

module Views
  module Rodauth
    class OtpSetup < Views::Rodauth::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        page_layout do
          render_auth_card(
            title: t('rodauth.views.otp_setup.page_title'),
            subtitle: t('rodauth.views.otp_setup.page_subtitle')
          ) do
            render_qr_code_section
            render_manual_entry_section
            render_setup_form
          end
        end
      end

      private

      def render_qr_code_section
        div(class: 'flex flex-col items-center space-y-4 pb-4') do
          div(class: 'rounded-2xl border border-outline-variant bg-surface-container-lowest p-6 shadow-sm') do
            div(class: 'w-48 h-48 flex items-center justify-center [&_img]:max-w-full [&_img]:max-h-full [&_svg]:max-w-full [&_svg]:max-h-full') do
              qr_html = rodauth.otp_qr_code
              safe(qr_html)
            end
          end
          p(class: 'text-center text-sm text-on-surface-variant font-medium') do
            t('rodauth.views.otp_setup.qr_hint')
          end
        end
      end

      def render_manual_entry_section
        details(class: 'group border-t border-outline-variant/30 pt-6') do
          summary(class: 'flex cursor-pointer list-none items-center gap-2 text-sm text-on-surface-variant font-bold transition-colors hover:text-primary') do
            chevron_icon
            span { t('rodauth.views.otp_setup.manual_entry_summary') }
          end

          div(class: 'mt-6 space-y-4') do
            render_secret_field
            render_provisioning_url
          end
        end
      end

      def chevron_icon
        render Icons::ChevronRight.new(size: 16, class: 'transition-transform group-open:rotate-90')
      end

      def render_secret_field
        div(class: 'space-y-3 rounded-2xl border border-outline-variant/50 bg-surface-container-low p-5') do
          label(class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant') { t('rodauth.views.otp_setup.secret_key') }
          div(class: 'flex items-center gap-3') do
            code(class: 'flex-1 break-all select-all font-mono text-sm font-bold text-foreground') do
              plain rodauth.otp_user_key
            end
            render_copy_button(rodauth.otp_user_key)
          end
        end
      end

      def render_provisioning_url
        div(class: 'space-y-3 rounded-2xl border border-outline-variant/50 bg-surface-container-low p-5') do
          label(class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant') { t('rodauth.views.otp_setup.provisioning_url') }
          code(class: 'block break-all select-all font-mono text-xs text-on-surface-variant') do
            plain rodauth.otp_provisioning_uri
          end
        end
      end

      def render_copy_button(text)
        button(
          type: 'button',
          class: 'p-2.5 rounded-xl bg-surface-container-highest text-primary transition-all hover:scale-110 active:scale-95',
          title: t('rodauth.views.otp_setup.copy_to_clipboard'),
          data: { action: 'click->clipboard#copy', clipboard_text_param: text }
        ) do
          render Icons::Copy.new(size: 18)
        end
      end

      def render_setup_form
        div(class: 'border-t border-outline-variant/30 pt-8') do
          render RubyUI::Form.new(
            action: rodauth.otp_setup_path,
            method: :post,
            class: 'space-y-6',
            data_turbo: 'false'
          ) do
            authenticity_token_field
            otp_secret_field
            render_password_field
            render_otp_code_field
            render_submit_button
          end
        end
      end

      def otp_secret_field
        input(type: 'hidden', name: rodauth.otp_setup_param, value: rodauth.otp_user_key)
        input(type: 'hidden', name: rodauth.otp_setup_raw_param, value: rodauth.otp_key) if rodauth.otp_keys_use_hmac?
      end

      def render_password_field
        render_m3_form_field(
          label: t('rodauth.views.otp_setup.current_password_label'),
          input_attrs: {
            type: :password,
            name: 'password',
            id: 'password',
            required: true,
            autocomplete: 'current-password',
            placeholder: t('rodauth.views.otp_setup.current_password_placeholder')
          }
        )
      end

      def render_otp_code_field
        render_m3_form_field(
          label: t('rodauth.views.otp_setup.auth_code_label'),
          input_attrs: {
            type: :text,
            name: rodauth.otp_auth_param,
            id: 'otp',
            required: true,
            autocomplete: 'one-time-code',
            inputmode: 'numeric',
            pattern: '[0-9]*',
            maxlength: 6,
            placeholder: t('rodauth.views.otp_setup.auth_code_placeholder')
          }
        )
      end

      def render_submit_button
        render_m3_submit_button(t('rodauth.views.otp_setup.submit'))
      end
    end
  end
end