# frozen_string_literal: true

module Components
  module Admin
    module Users
      # Renders a single user row in the users table
      class UserRow < Components::Base
        include Phlex::Rails::Helpers::FormWith

        attr_reader :user, :current_user

        def initialize(user:, current_user: nil)
          @user = user
          @current_user = current_user
          super()
        end

        def view_template
          row_class = user.active? ? '' : 'opacity-60'
          render RubyUI::TableRow.new(class: row_class, data: { user_id: user.id }) do
            render RubyUI::TableCell.new(class: 'font-medium') { user.name }
            render(RubyUI::TableCell.new { user.email_address })
            render RubyUI::TableCell.new(class: 'capitalize') { user.role }
            render(RubyUI::TableCell.new { render_status_badge })
            render RubyUI::TableCell.new(class: 'text-right') do
              div(class: 'flex gap-2 justify-end') do
                render RubyUI::Link.new(href: "/admin/users/#{user.id}/edit", variant: :outline, size: :sm) { t('admin.users.user_row.edit') }
                render_verification_button
                render_activation_button
              end
            end
          end
        end

        private

        def render_status_badge
          if user.active?
            render RubyUI::Badge.new(variant: :green) { t('admin.users.user_row.active') }
          else
            render RubyUI::Badge.new(variant: :red) { t('admin.users.user_row.inactive') }
          end
        end

        def render_activation_button
          is_current_user = user == current_user

          if user.active?
            if is_current_user
              render RubyUI::Button.new(variant: :destructive_outline, size: :sm, disabled: true) { t('admin.users.user_row.deactivate') }
            else
              render_deactivate_dialog
            end
          else
            render_activate_button
          end
        end

        def render_activate_button
          form_with(
            url: "/admin/users/#{user.id}/activate",
            method: :post,
            class: 'inline-block'
          ) do
            Button(
              type: :submit,
              variant: :success_outline,
              size: :sm
            ) { t('admin.users.user_row.activate') }
          end
        end

        def render_verification_button
          return render_verified_button if user.person&.account&.verified?

          form_with(
            url: "/admin/users/#{user.id}/verify",
            method: :post,
            class: 'inline-block'
          ) do
            Button(
              type: :submit,
              variant: :outline,
              size: :sm
            ) { t('admin.users.user_row.verify') }
          end
        end

        def render_verified_button
          Button(
            type: :button,
            variant: :outline,
            size: :sm,
            disabled: true
          ) { t('admin.users.user_row.verified') }
        end

        def render_deactivate_dialog
          render RubyUI::AlertDialog.new do
            render RubyUI::AlertDialogTrigger.new do
              Button(variant: :destructive_outline, size: :sm) do
                t('admin.users.user_row.deactivate')
              end
            end
            render RubyUI::AlertDialogContent.new do
              render RubyUI::AlertDialogHeader.new do
                render(RubyUI::AlertDialogTitle.new { t('admin.users.user_row.deactivate_dialog.title') })
                render RubyUI::AlertDialogDescription.new do
                  t('admin.users.user_row.deactivate_dialog.confirm', name: user.name)
                end
              end
              render RubyUI::AlertDialogFooter.new do
                render(RubyUI::AlertDialogCancel.new { t('admin.users.user_row.deactivate_dialog.cancel') })
                form_with(url: "/admin/users/#{user.id}", method: :delete, class: 'inline') do
                  Button(variant: :destructive, type: :submit) { t('admin.users.user_row.deactivate_dialog.submit') }
                end
              end
            end
          end
        end
      end
    end
  end
end
