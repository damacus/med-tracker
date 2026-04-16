# frozen_string_literal: true

module Views
  module Profiles
    class CloseAccountDialog < Views::Base
      include Phlex::Rails::Helpers::T

      def view_template
        div(class: 'flex items-start justify-between rounded-shape-lg border border-destructive/20 bg-popover p-4 shadow-elevation-1 transition-colors hover:border-destructive/30 hover:bg-tertiary-container/40') do
          render_account_info
          render_alert_dialog
        end
      end

      private

      def render_account_info
        div(class: 'flex-1') do
          h3(class: 'text-sm font-medium text-foreground') { t('profiles.close_account.title') }
          p(class: 'mt-1 text-sm text-on-surface-variant') { t('profiles.close_account.description') }
        end
      end

      def render_alert_dialog
        div(class: 'ml-4') do
          render AlertDialog.new do
            render AlertDialogTrigger.new do
              m3_button(variant: :destructive, size: :sm) { t('profiles.close_account.title') }
            end
            render AlertDialogContent.new do
              render_dialog_header
              render_dialog_footer
            end
          end
        end
      end

      def render_dialog_header
        render AlertDialogHeader.new do
          render(AlertDialogTitle.new { t('profiles.close_account.dialog_title') })
          render(AlertDialogDescription.new do
            t('profiles.close_account.dialog_description')
          end)
        end
      end

      def render_dialog_footer
        render AlertDialogFooter.new do
          render(AlertDialogCancel.new { t('profiles.close_account.cancel_button') })
          render_close_account_form
        end
      end

      def render_close_account_form
        render RubyUI::Form.new(action: view_context.rodauth.close_account_path, method: :post, class: 'space-y-3') do
          render_close_account_hidden_fields
          render_close_account_password_field
          m3_button(type: :submit, variant: :destructive, class: 'w-full') { t('profiles.close_account.confirm_button') }
        end
      end

      def render_close_account_hidden_fields
        additional_tags = view_context.rodauth.close_account_additional_form_tags
        safe(additional_tags) if additional_tags.present?
        input(type: 'hidden', name: 'authenticity_token', value: view_context.form_authenticity_token)
      end

      def render_close_account_password_field
        render RubyUI::FormField.new do
          render RubyUI::FormFieldLabel.new(for: 'close-account-password') { t('rodauth.views.change_login.password_label') }
          m3_input(
            type: :password,
            name: view_context.rodauth.password_param,
            id: 'close-account-password',
            required: true,
            autocomplete: 'current-password'
          )
        end
      end
    end
  end
end
