# frozen_string_literal: true

module Views
  module Rodauth
    class ResetPassword < Views::Rodauth::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        page_layout do
          render_auth_card(
            title: t('rodauth.views.reset_password.card_title'),
            subtitle: t('rodauth.views.reset_password.card_description')
          ) do
            flash_section
            render_reset_form
          end
        end
      end

      private

      def flash_section
        return if flash_message.blank?

        div(id: 'reset-flash') do
          render_m3_alert(flash_message, variant: flash_variant)
        end
      end

      def flash_message
        view_context.flash[:alert] || view_context.flash[:notice] || rodauth_error
      end

      def rodauth_error
        view_context.rodauth.field_error('password') ||
          view_context.rodauth.field_error('password-confirm')
      end

      def flash_variant
        view_context.flash[:alert].present? || rodauth_error.present? ? :destructive : :success
      end

      def render_reset_form
        render RubyUI::Form.new(
          action: view_context.rodauth.reset_password_path, method: :post,
          class: 'space-y-6', data_turbo: 'false'
        ) do
          authenticity_token_field
          key_field
          render_password_field
          render_password_confirm_field
          render_submit_button
        end
      end

      def key_field
        input(type: 'hidden', name: 'key', value: view_context.params[:key])
      end

      def render_password_field
        render_m3_form_field(
          label: t('rodauth.views.reset_password.new_password_label'),
          input_attrs: {
            type: :password,
            name: 'password',
            id: 'password',
            required: true,
            autofocus: true,
            autocomplete: 'new-password',
            placeholder: t('rodauth.views.reset_password.new_password_placeholder'),
            minlength: 12,
            maxlength: 72
          },
          error: view_context.rodauth.field_error('password')
        )
      end

      def render_password_confirm_field
        render_m3_form_field(
          label: t('rodauth.views.reset_password.confirm_password_label'),
          input_attrs: {
            type: :password,
            name: 'password-confirm',
            id: 'password-confirm',
            required: true,
            autocomplete: 'new-password',
            placeholder: t('rodauth.views.reset_password.confirm_password_placeholder'),
            minlength: 12,
            maxlength: 72
          },
          error: view_context.rodauth.field_error('password-confirm')
        )
      end

      def render_submit_button
        render_m3_submit_button(t('rodauth.views.reset_password.submit'))
      end
    end
  end
end
