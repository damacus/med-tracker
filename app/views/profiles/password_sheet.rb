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
          render Button.new(variant: :outline, size: :sm) { t('profiles.password_sheet.change_button') }
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
          render(SheetTitle.new { t('profiles.password_sheet.title') })
          render(SheetDescription.new { t('profiles.password_sheet.description') })
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
          render_password_field('current_password', t('profiles.password_sheet.current_password_label'), t('profiles.password_sheet.current_password_placeholder'))
          render_password_field('new_password', t('profiles.password_sheet.new_password_label'), t('profiles.password_sheet.new_password_placeholder'))
          render_password_field('password_confirmation', t('profiles.password_sheet.confirm_password_label'), t('profiles.password_sheet.confirm_password_placeholder'))
        end
      end

      def render_sheet_footer
        render SheetFooter.new do
          render Button.new(variant: :outline, data: { action: 'click->ruby-ui--sheet-content#close' }) { t('profiles.password_sheet.cancel') }
          render Button.new(type: :submit) { t('profiles.password_sheet.submit') }
        end
      end

      def render_password_field(name, label_text, placeholder)
        div do
          label(class: 'mb-2 block text-sm font-medium text-foreground') { label_text }
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
