# frozen_string_literal: true

module Views
  module Rodauth
    class ChangePassword < Views::Rodauth::Base
      def view_template
        page_layout do
          render_auth_card(title: rodauth.change_password_button) do
            flash_section
            render_change_password_form
          end
        end
      end

      private

      def flash_section
        return if flash_message.blank?

        render_m3_alert(flash_message)
      end

      def flash_message
        view_context.flash[:alert] ||
          rodauth.field_error(rodauth.password_param) ||
          rodauth.field_error(rodauth.new_password_param) ||
          rodauth.field_error(rodauth.password_confirm_param)
      end

      def render_change_password_form
        render RubyUI::Form.new(action: rodauth.change_password_path, method: :post, class: 'space-y-6',
                                data_turbo: 'false') do
          authenticity_token_field
          render_current_password_field
          render_new_password_field
          render_confirm_password_field
          render_submit_button
        end
      end

      def render_current_password_field
        render_m3_form_field(
          label: t('rodauth.views.change_password.password_label'),
          input_attrs: {
            type: :password,
            name: rodauth.password_param,
            id: 'password',
            required: true,
            autocomplete: 'current-password'
          },
          error: rodauth.field_error(rodauth.password_param)
        )
      end

      def render_new_password_field
        render_m3_form_field(
          label: t('rodauth.views.change_password.new_password_label'),
          input_attrs: {
            type: :password,
            name: rodauth.new_password_param,
            id: 'new-password',
            required: true,
            autocomplete: 'new-password'
          },
          error: rodauth.field_error(rodauth.new_password_param)
        )
      end

      def render_confirm_password_field
        render_m3_form_field(
          label: t('rodauth.views.change_password.confirm_password_label'),
          input_attrs: {
            type: :password,
            name: rodauth.password_confirm_param,
            id: 'password-confirm',
            required: true,
            autocomplete: 'new-password'
          },
          error: rodauth.field_error(rodauth.password_confirm_param)
        )
      end

      def render_submit_button
        render_m3_submit_button(rodauth.change_password_button)
      end
    end
  end
end
