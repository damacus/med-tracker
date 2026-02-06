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
            render RubyUI::TableCell.new(class: 'text-right space-x-2') do
              render RubyUI::Link.new(href: "/admin/users/#{user.id}/edit", variant: :outline, size: :sm) { 'Edit' }
              render_activation_button
            end
          end
        end

        private

        def render_status_badge
          if user.active?
            render RubyUI::Badge.new(variant: :green) { 'Active' }
          else
            render RubyUI::Badge.new(variant: :red) { 'Inactive' }
          end
        end

        def render_activation_button
          is_current_user = user == current_user

          if user.active?
            if is_current_user
              render RubyUI::Button.new(variant: :outline, size: :sm, disabled: true, class: 'text-red-600') { 'Deactivate' }
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
              variant: :outline,
              size: :sm,
              class: 'text-green-600 hover:text-green-500 border-green-600'
            ) { 'Activate' }
          end
        end

        def render_deactivate_dialog
          render RubyUI::AlertDialog.new do
            render RubyUI::AlertDialogTrigger.new do
              Button(variant: :outline, size: :sm, class: 'text-red-600 hover:bg-red-50 hover:text-red-700') do
                'Deactivate'
              end
            end
            render RubyUI::AlertDialogContent.new do
              render RubyUI::AlertDialogHeader.new do
                render(RubyUI::AlertDialogTitle.new { 'Deactivate User Account' })
                render RubyUI::AlertDialogDescription.new do
                  "Are you sure you want to deactivate #{user.name}'s account? They will no longer be able to sign in."
                end
              end
              render RubyUI::AlertDialogFooter.new do
                render(RubyUI::AlertDialogCancel.new { 'Cancel' })
                form_with(url: "/admin/users/#{user.id}", method: :delete, class: 'inline') do
                  Button(variant: :destructive, type: :submit) { 'Deactivate' }
                end
              end
            end
          end
        end
      end
    end
  end
end
