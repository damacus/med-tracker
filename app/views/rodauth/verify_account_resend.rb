# frozen_string_literal: true

module Views
  module Rodauth
    class VerifyAccountResend < Views::Rodauth::EmailFormBase
      private

      def page_subtitle
        'Resend your account verification email.'
      end

      def card_title
        'Resend Verification Email'
      end

      def card_description
        "Enter your email address and we'll send you a new verification link."
      end

      def form_action
        rodauth.verify_account_resend_path
      end

      def submit_label
        'Resend Verify Account Information'
      end

      def flash_id
        'verify-flash'
      end
    end
  end
end
