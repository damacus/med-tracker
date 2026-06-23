# frozen_string_literal: true

module Components
  module Layouts
    # Profile dropdown menu for authenticated users
    class ProfileMenu < Components::Base
      include Components::Layouts::CurrentUserContext
      include Components::Layouts::NavigationItems
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        render RubyUI::DropdownMenu.new do
          render RubyUI::DropdownMenuTrigger.new(class: 'w-full') do
            m3_button(variant: :outlined, class: 'gap-2') do
              if current_user&.person
                render Components::Shared::PersonAvatar.new(person: current_user.person, size: :xs)
              end
              span { current_user_name || t('layouts.profile_menu.account') }
            end
          end
          render RubyUI::DropdownMenuContent.new do
            render(RubyUI::DropdownMenuLabel.new { t('layouts.profile_menu.my_account') })
            render RubyUI::DropdownMenuSeparator.new
            render RubyUI::DropdownMenuItem.new(href: household_navigation_path(:dashboard_path)) do
              t('layouts.profile_menu.dashboard')
            end
            render RubyUI::DropdownMenuItem.new(href: profile_navigation_item[:path]) { t('layouts.profile_menu.profile') }
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
