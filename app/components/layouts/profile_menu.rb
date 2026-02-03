# frozen_string_literal: true

module Components
  module Layouts
    # Profile dropdown menu for authenticated users
    class ProfileMenu < Components::Base
      attr_reader :current_user

      def initialize(current_user: nil)
        @current_user = current_user
        super()
      end

      def view_template
        render RubyUI::DropdownMenu.new do
          render RubyUI::DropdownMenuTrigger.new(class: 'w-full') do
            Button(variant: :outline) { current_user_name }
          end
          render RubyUI::DropdownMenuContent.new do
            render(RubyUI::DropdownMenuLabel.new { 'My Account' })
            render RubyUI::DropdownMenuSeparator.new
            render RubyUI::DropdownMenuItem.new(href: root_path) { 'Dashboard' }
            render RubyUI::DropdownMenuItem.new(href: profile_path) { 'Profile' }
            render_admin_menu_item if user_is_admin?
            render RubyUI::DropdownMenuSeparator.new
            render_logout_menu_item
          end
        end
      end

      private

      def render_admin_menu_item
        render RubyUI::DropdownMenuItem.new(href: admin_root_path) { 'Administration' }
      end

      def render_logout_menu_item
        render RubyUI::DropdownMenuItem.new(
          href: '/logout',
          data: { turbo_method: :post }
        ) { 'Logout' }
      end

      def current_user_name
        current_user&.name || 'Account'
      end

      def user_is_admin?
        current_user&.administrator? || false
      end
    end
  end
end
