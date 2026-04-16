# frozen_string_literal: true

module Components
  module Layouts
    # Profile dropdown menu for authenticated users
    class ProfileMenu < Components::Base
      include Components::Layouts::CurrentUserContext
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        render RubyUI::DropdownMenu.new do
          render RubyUI::DropdownMenuTrigger.new(class: 'w-full') do
            m3_button(variant: :outlined) { current_user_name || t('layouts.profile_menu.account') }
          end
          render RubyUI::DropdownMenuContent.new do
            render(RubyUI::DropdownMenuLabel.new { t('layouts.profile_menu.my_account') })
            render RubyUI::DropdownMenuSeparator.new
            render RubyUI::DropdownMenuItem.new(href: root_path) { t('layouts.profile_menu.dashboard') }
            render RubyUI::DropdownMenuItem.new(href: profile_path) { t('layouts.profile_menu.profile') }
            render_admin_menu_item if user_is_admin?
            render RubyUI::DropdownMenuSeparator.new
            render_logout_menu_item
          end
        end
      end

      private

      def render_admin_menu_item
        render RubyUI::DropdownMenuItem.new(href: admin_root_path) { t('layouts.profile_menu.administration') }
      end

      def render_logout_menu_item
        render RubyUI::DropdownMenuItem.new(
          href: '/logout',
          data: { turbo_method: :post }
        ) { t('layouts.profile_menu.logout') }
      end
    end
  end
end
