# frozen_string_literal: true

module Views
  module Rodauth
    class OtpAuth < Views::Rodauth::Base
      def view_template
        page_layout do
          render_page_header(
            title: t('rodauth.views.otp_auth.page_title'),
            subtitle: t('rodauth.views.otp_auth.page_subtitle')
          )
          form_section
        end
      end

      private

      def form_section
        render_auth_card(
          title: t('rodauth.views.otp_auth.card_title'),
          subtitle: t('rodauth.views.otp_auth.card_description')
        ) do
          flash_section
          otp_form
        end
      end

      def flash_section
        return if flash_message.blank?

        render_m3_alert(flash_message)
      end

      def flash_message
        view_context.flash[:alert] || view_context.rodauth.field_error(rodauth.otp_auth_param)
      end

      def otp_form
        render RubyUI::Form.new(
          action: rodauth.otp_auth_path, method: :post,
          class: 'space-y-6', data_turbo: 'false'
        ) do
          additional_tags = rodauth.otp_auth_additional_form_tags
          safe(additional_tags) if additional_tags.present?
          authenticity_token_field
          otp_code_field
          submit_button
        end
      end

      def otp_code_field
        render_m3_form_field(
          label: rodauth.otp_auth_label,
          input_attrs: {
            type: :text,
            name: rodauth.otp_auth_param,
            id: 'otp-auth-code',
            required: true,
            autocomplete: 'one-time-code',
            inputmode: 'numeric',
            pattern: '[0-9]*',
            maxlength: 6,
            placeholder: t('rodauth.views.otp_auth.code_placeholder')
          },
          error: view_context.rodauth.field_error(rodauth.otp_auth_param)
        )
      end

      def submit_button
        render_m3_submit_button(rodauth.otp_auth_button)
      end
    end
  end
end