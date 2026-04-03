# frozen_string_literal: true

module Views
  module Rodauth
    class ChangePassword < Views::Rodauth::Base
      def view_template
        page_layout do
          form_section
        end
      end

      private

      def flash_section
        return if flash_message.blank?

        render RubyUI::Alert.new(variant: :destructive) do
          plain(flash_message)
        end
      end

      def flash_message
        view_context.flash[:alert] ||
          rodauth.field_error(rodauth.password_param) ||
          rodauth.field_error(rodauth.new_password_param) ||
          rodauth.field_error(rodauth.password_confirm_param)
      end

      def form_section
        render RubyUI::Card.new(class: card_classes) do
          render RubyUI::CardContent.new(class: 'space-y-6 p-6 sm:p-8') do
            flash_section
            change_password_form
          end
        end
      end

      def change_password_form
        render RubyUI::Form.new(action: rodauth.change_password_path, method: :post, class: 'space-y-6',
                                data_turbo: 'false') do
          authenticity_token_field
          current_password_field
          new_password_field
          confirm_password_field
          submit_button
        end
      end

      def current_password_field
        render RubyUI::FormField.new do
          render RubyUI::FormFieldLabel.new(for: 'password') { t('rodauth.views.change_password.password_label') }
          render RubyUI::Input.new(
            type: :password,
            name: rodauth.password_param,
            id: 'password',
            required: true,
            autocomplete: 'current-password'
          )
          error = rodauth.field_error(rodauth.password_param)
          p(class: 'mt-1 text-sm text-error') { error } if error.present?
        end
      end

      def new_password_field
        render RubyUI::FormField.new do
          render RubyUI::FormFieldLabel.new(for: 'new-password') { t('rodauth.views.change_password.new_password_label') }
          render RubyUI::Input.new(
            type: :password,
            name: rodauth.new_password_param,
            id: 'new-password',
            required: true,
            autocomplete: 'new-password'
          )
          error = rodauth.field_error(rodauth.new_password_param)
          p(class: 'mt-1 text-sm text-error') { error } if error.present?
        end
      end

      def confirm_password_field
        render RubyUI::FormField.new do
          render RubyUI::FormFieldLabel.new(for: 'password-confirm') { t('rodauth.views.change_password.confirm_password_label') }
          render RubyUI::Input.new(
            type: :password,
            name: rodauth.password_confirm_param,
            id: 'password-confirm',
            required: true,
            autocomplete: 'new-password'
          )
          error = rodauth.field_error(rodauth.password_confirm_param)
          p(class: 'mt-1 text-sm text-error') { error } if error.present?
        end
      end

      def submit_button
        render RubyUI::Button.new(type: :submit, variant: :primary, size: :md, class: 'w-full') do
          rodauth.change_password_button
        end
      end
    end
  end
end
