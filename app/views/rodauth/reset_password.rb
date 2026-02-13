# frozen_string_literal: true

module Views
  module Rodauth
    class ResetPassword < Views::Rodauth::AuthFormBase
      private

      def page_subtitle = 'Create a new password for your account.'
      def card_title = 'Set New Password'
      def card_description = 'Enter your new password below.'
      def flash_id = 'reset-flash'

      def flash_message
        super || rodauth_error
      end

      def flash_variant
        view_context.flash[:alert].present? || rodauth_error.present? ? :destructive : :success
      end

      def render_form
        render RubyUI::Form.new(action: rodauth.reset_password_path, method: :post, class: 'space-y-6', data_turbo: 'false') do
          authenticity_token_field
          key_field
          password_field
          password_confirm_field
          submit_button('Reset Password')
        end
      end

      def rodauth_error
        rodauth.field_error('password') ||
          rodauth.field_error('password-confirm')
      end

      def key_field
        input(type: 'hidden', name: 'key', value: view_context.params[:key])
      end

      def password_field
        render_form_field(
          label: 'New Password',
          input_attrs: {
            type: :password,
            name: 'password',
            id: 'password',
            required: true,
            autofocus: true,
            autocomplete: 'new-password',
            placeholder: 'Enter your new password (min 12 characters)',
            minlength: 12,
            maxlength: 72
          },
          error: rodauth.field_error('password')
        )
      end

      def password_confirm_field
        render_form_field(
          label: 'Confirm New Password',
          input_attrs: {
            type: :password,
            name: 'password-confirm',
            id: 'password-confirm',
            required: true,
            autocomplete: 'new-password',
            placeholder: 'Confirm your new password',
            minlength: 12,
            maxlength: 72
          },
          error: rodauth.field_error('password-confirm')
        )
      end
    end
  end
end
