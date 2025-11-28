# frozen_string_literal: true

module Views
  module Profiles
    class CloseAccountDialog < Views::Base
      include Phlex::Rails::Helpers::T

      def view_template
        div(class: 'flex items-start justify-between p-4 border border-slate-200 rounded-lg hover:bg-slate-50 transition-colors') do
          render_account_info
          render_alert_dialog
        end
      end

      private

      def render_account_info
        div(class: 'flex-1') do
          h3(class: 'text-sm font-medium text-slate-900') { t('profiles.close_account.title') }
          p(class: 'text-sm text-slate-600 mt-1') { t('profiles.close_account.description') }
        end
      end

      def render_alert_dialog
        div(class: 'ml-4') do
          render AlertDialog.new do
            render AlertDialogTrigger.new do
              render Button.new(variant: :destructive, size: :sm) { t('profiles.close_account.title') }
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
          render AlertDialogAction.new(
            data: { turbo_method: :delete, turbo_confirm: false },
            href: '/close-account'
          ) { t('profiles.close_account.confirm_button') }
        end
      end
    end
  end
end
