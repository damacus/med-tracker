# frozen_string_literal: true

module Views
  module Rodauth
    class OtpDisable < Views::Rodauth::Base
      def view_template
        page_layout do
          render_auth_card(
            title: t('rodauth.views.otp_disable.page_title'),
            subtitle: t('rodauth.views.otp_disable.page_subtitle')
          ) do
            render_otp_disable_form
          end
        end
      end

      private

      def render_otp_disable_form
        render RubyUI::Form.new(action: rodauth.otp_disable_path, method: :post, class: 'space-y-6', data_turbo: 'false') do
          additional_tags = rodauth.otp_disable_additional_form_tags
          safe(additional_tags) if additional_tags.present?
          authenticity_token_field
          render_password_field if rodauth.two_factor_modifications_require_password?
          render_submit_button
        end
      end

      def render_password_field
        render_m3_form_field(
          label: t('rodauth.views.otp_disable.password_label'),
          input_attrs: {
            type: :password,
            name: 'password',
            id: 'password',
            required: true,
            autocomplete: 'current-password',
            placeholder: t('rodauth.views.otp_disable.password_placeholder')
          }
        )
      end

      def render_submit_button
        m3_button(type: :submit, variant: :destructive, size: :lg, class: 'w-full py-6 font-bold shadow-lg shadow-error/20') do
          rodauth.otp_disable_button
        end
      end
    end
  end
end