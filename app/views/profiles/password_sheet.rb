# frozen_string_literal: true

module Views
  module Profiles
    class PasswordSheet < Views::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::Routes

      def view_template
        render Sheet.new do
          render_sheet_trigger
          render_sheet_content
        end
      end

      private

      def render_sheet_trigger
        render SheetTrigger.new do
          render Button.new(variant: :outline, size: :sm) { 'Change' }
        end
      end

      def render_sheet_content
        render SheetContent.new(class: 'sm:max-w-sm') do
          render_sheet_header
          render_password_form
        end
      end

      def render_sheet_header
        render SheetHeader.new do
          render(SheetTitle.new { 'Change Password' })
          render(SheetDescription.new { 'Update your password to keep your account secure.' })
        end
      end

      def render_password_form
        form_with(url: '/change-password', method: :post) do
          render SheetMiddle.new do
            render_password_fields
          end
          render_sheet_footer
        end
      end

      def render_password_fields
        div(class: 'space-y-4') do
          render_password_field('current_password', 'Current Password', 'Enter current password')
          render_password_field('new_password', 'New Password', 'Enter new password')
          render_password_field('password_confirmation', 'Confirm New Password', 'Confirm new password')
        end
      end

      def render_sheet_footer
        render SheetFooter.new do
          render Button.new(variant: :outline, data: { action: 'click->ruby-ui--sheet-content#close' }) { 'Cancel' }
          render Button.new(type: :submit) { 'Update Password' }
        end
      end

      def render_password_field(name, label_text, placeholder)
        div do
          label(class: 'text-sm font-medium text-slate-900 mb-2 block') { label_text }
          render Input.new(
            type: 'password',
            name: name,
            placeholder: placeholder,
            required: true
          )
        end
      end
    end
  end
end
