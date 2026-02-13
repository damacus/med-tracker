# frozen_string_literal: true

module Views
  module Rodauth
    class CreateAccount < Views::Rodauth::AuthFormBase
      private

      def page_subtitle = 'Create your account to start tracking medications safely.'
      def card_title = 'Create Account'
      def card_description = 'Fill in your details to create a new account.'
      def flash_id = 'signup-flash'

      def flash_message
        super || rodauth_error
      end

      def flash_variant
        view_context.flash[:alert].present? || rodauth_error.present? ? :destructive : :success
      end

      def render_form
        render RubyUI::Form.new(action: rodauth.create_account_path, method: :post, class: 'space-y-6', data_turbo: 'false') do
          authenticity_token_field
          name_field
          date_of_birth_field
          email_field
          password_field
          password_confirm_field
          submit_button('Create Account')
        end
      end

      def rodauth_error
        rodauth.field_error('login') ||
          rodauth.field_error('password') ||
          rodauth.field_error('password-confirm')
      end

      def name_field
        render_form_field(
          label: 'Name',
          input_attrs: {
            type: :text,
            name: 'name',
            id: 'name',
            required: true,
            autofocus: true,
            autocomplete: 'name',
            placeholder: 'Enter your full name',
            value: view_context.params[:name]
          },
          error: rodauth.field_error('name')
        )
      end

      def date_of_birth_field
        render_form_field(
          label: 'Date of birth',
          input_attrs: {
            type: :date,
            name: 'date_of_birth',
            id: 'date_of_birth',
            required: true,
            autocomplete: 'bday',
            value: view_context.params[:date_of_birth]
          },
          error: rodauth.field_error('date_of_birth')
        )
      end

      def email_field
        render_form_field(
          label: 'Email',
          input_attrs: {
            type: :email,
            name: 'email',
            id: 'email',
            required: true,
            autocomplete: 'email',
            placeholder: 'Enter your email address',
            value: view_context.params[:email]
          },
          error: rodauth.field_error('login')
        )
      end

      def password_field
        render_form_field(
          label: 'Password',
          input_attrs: {
            type: :password,
            name: 'password',
            id: 'password',
            required: true,
            autocomplete: 'new-password',
            placeholder: 'Create a password (min 12 characters)',
            minlength: 12,
            maxlength: 72
          },
          error: rodauth.field_error('password')
        )
      end

      def password_confirm_field
        render_form_field(
          label: 'Confirm Password',
          input_attrs: {
            type: :password,
            name: 'password-confirm',
            id: 'password-confirm',
            required: true,
            autocomplete: 'new-password',
            placeholder: 'Confirm your password',
            minlength: 12,
            maxlength: 72
          },
          error: rodauth.field_error('password-confirm')
        )
      end

      def render_other_options
        div(class: 'space-y-3 border-t border-slate-200 pt-6') do
          h3(class: 'text-sm font-medium text-slate-700') { 'Already have an account?' }
          div(class: 'flex flex-col gap-2 text-sm') do
            render RubyUI::Link.new(href: rodauth.login_path, variant: :link) do
              'Sign in to your account'
            end
          end
        end
      end
    end
  end
end
