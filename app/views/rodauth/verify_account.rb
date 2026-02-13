# frozen_string_literal: true

module Views
  module Rodauth
    class VerifyAccount < Views::Rodauth::AuthFormBase
      private

      def page_subtitle = 'Verify your account to complete registration.'
      def card_title = 'Verify Your Account'
      def card_description = 'Click the button below to verify your email address and activate your account.'
      def flash_id = 'verify-flash'

      def render_form
        render RubyUI::Form.new(action: rodauth.verify_account_path, method: :post, class: 'space-y-6', data_turbo: 'false') do
          authenticity_token_field
          key_field
          submit_button('Verify Account')
        end
      end

      def key_field
        input(type: 'hidden', name: 'key', value: view_context.params[:key])
      end
    end
  end
end
