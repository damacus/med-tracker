# frozen_string_literal: true

module Views
  module Rodauth
    class ResetPassword < Views::Rodauth::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        page_layout do
          render_page_header(
            title: t('app.name'),
            subtitle: t('rodauth.views.reset_password.page_subtitle')
          )
          form_section
        end
      end

      private

      def flash_section
        return if flash_message.blank?

        div(id: 'reset-flash') do
          render RubyUI::Alert.new(variant: flash_variant) do
            plain(flash_message)
          end
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

      def form_section
        render RubyUI::Card.new(class: card_classes) do
          render_card_header
          render_card_content
        end
      end

      def render_card_header
        render RubyUI::CardHeader.new(class: 'space-y-2 bg-white/60') do
          render RubyUI::CardTitle.new(class: 'text-2xl font-semibold text-slate-900') { t('rodauth.views.reset_password.card_title') }
          render RubyUI::CardDescription.new(class: 'text-base text-slate-600') do
            plain t('rodauth.views.reset_password.card_description')
          end
        end
      end

      def render_card_content
        render RubyUI::CardContent.new(class: 'space-y-6 p-6 sm:p-8') do
          flash_section
          render_reset_form
        end
      end

      def render_reset_form
        render RubyUI::Form.new(action: view_context.rodauth.reset_password_path, method: :post, class: 'space-y-6', data_turbo: 'false') do
          authenticity_token_field
          key_field
          password_field
          password_confirm_field
          submit_button
        end
      end

      def key_field
        input(type: 'hidden', name: 'key', value: view_context.params[:key])
      end

      def password_field
        render RubyUI::FormField.new do
          render RubyUI::FormFieldLabel.new(for: 'password') { t('rodauth.views.reset_password.new_password_label') }
          render RubyUI::Input.new(
            type: :password,
            name: 'password',
            id: 'password',
            required: true,
            autofocus: true,
            autocomplete: 'new-password',
            placeholder: t('rodauth.views.reset_password.new_password_placeholder'),
            minlength: 12,
            maxlength: 72
          )
          error = view_context.rodauth.field_error('password')
          p(class: 'text-sm text-red-600 mt-1') { error } if error.present?
        end
      end

      def password_confirm_field
        render RubyUI::FormField.new do
          render RubyUI::FormFieldLabel.new(for: 'password-confirm') { t('rodauth.views.reset_password.confirm_password_label') }
          render RubyUI::Input.new(
            type: :password,
            name: 'password-confirm',
            id: 'password-confirm',
            required: true,
            autocomplete: 'new-password',
            placeholder: t('rodauth.views.reset_password.confirm_password_placeholder'),
            minlength: 12,
            maxlength: 72
          )
          error = view_context.rodauth.field_error('password-confirm')
          p(class: 'text-sm text-red-600 mt-1') { error } if error.present?
        end
      end

      def submit_button
        render RubyUI::Button.new(type: :submit, variant: :primary, size: :md, class: 'w-full') do
          t('rodauth.views.reset_password.submit')
        end
      end
    end
  end
end
