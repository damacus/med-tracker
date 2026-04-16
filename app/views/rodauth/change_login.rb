# frozen_string_literal: true

module Views
  module Rodauth
    class ChangeLogin < Views::Rodauth::Base
      def view_template
        page_layout do
          render_auth_card(title: rodauth.change_login_button) do
            flash_section
            render_change_login_form
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
          rodauth.field_error(rodauth.login_param) ||
          rodauth.field_error(rodauth.password_param)
      end

      def render_change_login_form
        render RubyUI::Form.new(action: rodauth.change_login_path, method: :post, class: 'space-y-6',
                                data_turbo: 'false') do
          authenticity_token_field
          render_login_field
          render_password_field
          render_submit_button
        end
      end

      def render_login_field
        render_m3_form_field(
          label: t('rodauth.views.change_login.new_login_label'),
          input_attrs: {
            type: :email,
            name: rodauth.login_param,
            id: 'login',
            required: true,
            autocomplete: 'username'
          },
          error: rodauth.field_error(rodauth.login_param)
        )
      end

      def render_password_field
        render_m3_form_field(
          label: t('rodauth.views.change_login.password_label'),
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

      def render_submit_button
        render_m3_submit_button(rodauth.change_login_button)
      end
    end
  end
end