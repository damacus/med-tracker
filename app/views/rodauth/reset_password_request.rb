# frozen_string_literal: true

module Views
  module Rodauth
    class ResetPasswordRequest < Views::Rodauth::EmailFormBase
      private

      def page_subtitle
        'Reset your password to regain access to your account.'
      end

      def card_title
        'Reset Password'
      end

      def card_description
        "Enter your email address and we'll send you a link to reset your password."
      end

      def form_action
        rodauth.reset_password_request_path
      end

      def submit_label
        'Request Password Reset'
      end

      def flash_id
        'reset-flash'
      end
    end
  end
end
