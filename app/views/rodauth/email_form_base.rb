# frozen_string_literal: true

module Views
  module Rodauth
    class EmailFormBase < Views::Rodauth::AuthFormBase
      private

      def form_action
        raise NotImplementedError
      end

      def submit_label
        raise NotImplementedError
      end

      def render_form
        render RubyUI::Form.new(action: form_action, method: :post, class: 'space-y-6', data_turbo: 'false') do
          authenticity_token_field
          email_field
          submit_button(submit_label)
        end
      end

      def email_field
        render_form_field(
          label: 'Email address',
          input_attrs: {
            type: :email,
            name: 'email',
            id: 'email',
            required: true,
            autofocus: true,
            autocomplete: 'email',
            placeholder: 'Enter your email address',
            value: view_context.params[:email]
          }
        )
      end

      def render_other_options
        div(class: 'space-y-3 border-t border-slate-200 pt-6') do
          h3(class: 'text-sm font-medium text-slate-700') { 'Other Options' }
          div(class: 'flex flex-col gap-2 text-sm') do
            render RubyUI::Link.new(href: rodauth.login_path, variant: :link) do
              'Back to Login'
            end
            render RubyUI::Link.new(href: rodauth.create_account_path, variant: :link) do
              'Create a New Account'
            end
          end
        end
      end
    end
  end
end
