# frozen_string_literal: true

module Views
  module Rodauth
    module LoginInputAttrsSupport
      private

      def email_input_attrs
        {
          type: :email,
          name: "email",
          id: "email",
          required: true,
          autofocus: true,
          autocomplete: "username webauthn",
          placeholder: t("sessions.login.email_placeholder"),
          value: view_context.params[:email]
        }
      end

      def password_input_attrs
        {
          type: :password,
          name: "password",
          id: "password",
          required: true,
          autocomplete: "current-password",
          placeholder: t("sessions.login.password_placeholder"),
          maxlength: 72
        }
      end
    end
  end
end
