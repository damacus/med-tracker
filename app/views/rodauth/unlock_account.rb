# frozen_string_literal: true

module Views
  module Rodauth
    class UnlockAccount < Views::Rodauth::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        page_layout do
          render_auth_card(
            title: rodauth.unlock_account_button,
            subtitle: t('rodauth.unlock_account.instruction')
          ) do
            flash_section
            render_explanatory_text
            render_unlock_form
          end
        end
      end

      private

      def flash_section
        return if flash_message.blank?

        div(id: 'unlock-account-flash') do
          render_m3_alert(flash_message)
        end
      end

      def flash_message
        view_context.flash[:alert]
      end

      def render_explanatory_text
        div(class: 'text-sm leading-6 text-on-surface-variant font-medium') do
          safe(rodauth.unlock_account_explanatory_text)
        end
      end

      def render_unlock_form
        render RubyUI::Form.new(
          action: rodauth.unlock_account_path, method: :post,
          class: 'space-y-6', data_turbo: 'false'
        ) do
          additional_tags = rodauth.unlock_account_additional_form_tags
          safe(additional_tags) if additional_tags.present?
          authenticity_token_field
          key_field
          render_password_field if rodauth.unlock_account_requires_password?
          render_submit_button
        end
      end

      def key_field
        key = view_context.params[rodauth.unlock_account_key_param]
        input(type: 'hidden', name: rodauth.unlock_account_key_param, value: key) if key.present?
      end

      def render_password_field
        render_m3_form_field(
          label: t('sessions.login.password_label'),
          input_attrs: {
            type: :password,
            name: rodauth.password_param,
            id: 'password',
            required: true,
            autocomplete: 'current-password',
            placeholder: t('sessions.login.password_placeholder')
          }
        )
      end

      def render_submit_button
        render_m3_submit_button(rodauth.unlock_account_button)
      end
    end
  end
end