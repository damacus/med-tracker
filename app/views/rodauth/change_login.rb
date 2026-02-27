# frozen_string_literal: true

module Views
  module Rodauth
    class ChangeLogin < Views::Rodauth::Base
      def view_template
        page_layout do
          form_section
        end
      end

      private

      def form_section
        render RubyUI::Card.new(class: card_classes) do
          render RubyUI::CardContent.new(class: 'space-y-6 p-6 sm:p-8') do
            change_login_form
          end
        end
      end

      def change_login_form
        render RubyUI::Form.new(action: rodauth.change_login_path, method: :post, class: 'space-y-6',
                                data_turbo: 'false') do
          authenticity_token_field
          login_field
          password_field
          submit_button
        end
      end

      def login_field
        render RubyUI::FormField.new do
          render RubyUI::FormFieldLabel.new(for: 'login') { t('rodauth.views.change_login.new_login_label') }
          render RubyUI::Input.new(
            type: :email,
            name: rodauth.login_param,
            id: 'login',
            required: true,
            autocomplete: 'username'
          )
        end
      end

      def password_field
        render RubyUI::FormField.new do
          render RubyUI::FormFieldLabel.new(for: 'password') { t('rodauth.views.change_login.password_label') }
          render RubyUI::Input.new(
            type: :password,
            name: rodauth.password_param,
            id: 'password',
            required: true,
            autocomplete: 'current-password'
          )
        end
      end

      def submit_button
        render RubyUI::Button.new(type: :submit, variant: :primary, size: :md, class: 'w-full') do
          rodauth.change_login_button
        end
      end
    end
  end
end
