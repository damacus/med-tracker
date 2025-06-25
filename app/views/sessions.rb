# frozen_string_literal: true

module Views
  module Sessions
    class New < Views::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::LinkTo

      def initialize(params: {})
        @params = params
      end

      def view_template
        # Main login form using Phlex syntax
        form_with(url: session_path) do |form|
          form.label(:email_address)
          br
          form.email_field(
            :email_address,
            required: true,
            autofocus: true,
            autocomplete: "username",
            placeholder: "Enter your email address",
            value: @params[:email_address]
          )
          br

          form.label(:password)
          br
          form.password_field(
            :password,
            required: true,
            autocomplete: "current-password",
            placeholder: "Enter your password",
            maxlength: 72
          )
          br

          form.submit("Sign in")
        end

        br

        # Password recovery link
        link_to("Forgot password?", new_password_path)
      end
    end
  end
end
