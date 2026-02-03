# frozen_string_literal: true

module Components
  module Layouts
    # Navigation component that renders differently based on authentication state
    class Navigation < Components::Base
      # Include additional helpers needed for this component
      include Phlex::Rails::Helpers::ButtonTo
      include Phlex::Rails::Helpers::LinkTo

      # Initialize with optional current_user parameter (useful for testing)
      # @param [User, nil] current_user - The current user or nil if not authenticated
      def initialize(current_user: nil)
        @current_user = current_user
        super() # Initialize parent class
      end

      # Main template method that renders the navigation bar
      def view_template
        header(class: 'sticky top-0 z-40 w-full border-b bg-background/95 backdrop-blur ' \
                      'supports-[backdrop-filter]:bg-background/60') do
          nav(class: 'nav') do
            div(class: 'nav__container flex h-16 items-center justify-between px-4') do
              # Left side with brand and desktop navigation links
              div(class: 'nav__left flex items-center gap-6') do
                render_mobile_menu if authenticated?
                render_brand
                render_desktop_navigation_menu if authenticated?
              end

              # Right side with authentication actions (hidden on mobile if authenticated)
              div(class: 'nav__right flex items-center gap-4') do
                render_auth_actions
              end
            end
          end
        end

        render_bottom_navigation if authenticated?
      end

      private

      # Check if user is authenticated
      # @return [Boolean] true if user is authenticated, false otherwise
      def authenticated?
        # Always check the current authentication state via view context
        # Don't rely on cached @current_user prop for authentication state
        view_context.current_user.present?
      end

      # Render the brand/logo section
      def render_brand
        div(class: 'nav__brand') do
          link_to('MedTracker', root_path, class: 'nav__brand-link')
        end
      end

      # Render the mobile hamburger menu using RubyUI::Sheet
      def render_mobile_menu
        div(class: 'md:hidden') do
          render RubyUI::Sheet.new do
            render RubyUI::SheetTrigger.new do
              render RubyUI::Button.new(variant: :ghost, size: :icon, aria: { label: 'Open menu' }) do
                render Icons::Menu.new(class: 'h-6 w-6')
              end
            end

            render RubyUI::SheetContent.new(side: :left, class: 'w-[300px] sm:w-[400px]') do
              render RubyUI::SheetHeader.new do
                render(RubyUI::SheetTitle.new { 'MedTracker' })
              end

              div(class: 'grid gap-4 py-4') do
                render_mobile_navigation_links
              end

              render RubyUI::SheetFooter.new(class: 'mt-auto') do
                render_mobile_auth_actions
              end
            end
          end
        end
      end

      def render_mobile_navigation_links
        div(class: 'flex flex-col gap-1') do
          render_mobile_nav_link(medicines_path, Icons::Pill, 'Medicines', active_path?('/medicines'))
          render_mobile_nav_link(people_path, Icons::Users, 'People', active_path?('/people'))
          render_mobile_nav_link(medicine_finder_path, Icons::Search, 'Medicine Finder',
                                 active_path?('/medicine_finder'))
        end
      end

      def render_mobile_nav_link(path, icon_class, label, active)
        base_classes = 'justify-start gap-4 px-4 min-h-[48px] w-full'
        active_classes = active ? 'bg-accent text-accent-foreground font-semibold' : ''

        render RubyUI::Link.new(
          href: path,
          variant: :ghost,
          size: :xl,
          class: "#{base_classes} #{active_classes}",
          aria: { current: active ? 'page' : nil }
        ) do
          render icon_class.new(class: 'h-6 w-6')
          plain label
        end
      end

      def render_mobile_auth_actions
        div(class: 'flex flex-col gap-1 w-full') do
          render RubyUI::Link.new(
            href: profile_path,
            variant: :ghost,
            size: :xl,
            class: 'justify-start gap-4 px-4 min-h-[48px] w-full'
          ) do
            render Icons::User.new(class: 'h-6 w-6')
            plain 'Profile'
          end
          if user_is_admin?
            render RubyUI::Link.new(
              href: admin_root_path,
              variant: :ghost,
              size: :xl,
              class: 'justify-start gap-4 px-4 min-h-[48px] w-full'
            ) do
              render Icons::Settings.new(class: 'h-6 w-6')
              plain 'Administration'
            end
          end
          form(action: '/logout', method: 'post') do
            input(type: 'hidden', name: 'authenticity_token', value: view_context.form_authenticity_token)
            render RubyUI::Button.new(
              type: :submit,
              variant: :ghost,
              size: :xl,
              class: 'w-full justify-start gap-4 px-4 min-h-[48px] text-destructive hover:text-destructive'
            ) do
              render Icons::LogOut.new(class: 'h-6 w-6')
              plain 'Logout'
            end
          end
        end
      end

      # Render navigation menu for desktop
      def render_desktop_navigation_menu
        div(class: 'hidden md:flex items-center gap-6') do
          link_to('Medicines', medicines_path,
                  class: 'nav__link text-sm font-medium transition-colors hover:text-primary')
          link_to('People', people_path, class: 'nav__link text-sm font-medium transition-colors hover:text-primary')
          link_to('Medicine Finder', medicine_finder_path,
                  class: 'nav__link text-sm font-medium transition-colors hover:text-primary')
        end
      end

      # Render bottom navigation bar for mobile
      def render_bottom_navigation
        nav(class: 'mobile-nav fixed bottom-0 left-0 z-50 w-full h-16 bg-background border-t md:hidden ' \
                   'flex items-center justify-around px-2 pb-safe',
            role: 'navigation',
            aria: { label: 'Mobile navigation' }) do
          render_bottom_nav_item(root_path, Icons::Home, 'Home', active_path?('/'))
          render_bottom_nav_item(medicines_path, Icons::Pill, 'Medicines', active_path?('/medicines'))
          render_bottom_nav_item(people_path, Icons::Users, 'People', active_path?('/people'))
          render_bottom_nav_item(profile_path, Icons::User, 'Profile', active_path?('/profile'))
        end
      end

      def render_bottom_nav_item(path, icon_class, label, active)
        base_classes = 'flex flex-col items-center justify-center gap-0.5 min-w-[64px] min-h-[44px] ' \
                       'rounded-lg transition-colors active:bg-accent'
        state_classes = active ? 'text-primary' : 'text-muted-foreground hover:text-primary'

        link_to(path, class: "#{base_classes} #{state_classes}", aria: { current: active ? 'page' : nil }) do
          render icon_class.new(class: "h-5 w-5#{' scale-110' if active}")
          span(class: "text-[10px] font-medium#{' font-semibold' if active}") { label }
          div(class: 'h-0.5 w-4 rounded-full bg-primary mt-0.5') if active
        end
      end

      def active_path?(path)
        current_path = view_context.request.path
        return current_path == '/' if path == '/'

        current_path.start_with?(path)
      end

      # Render authentication actions (login or profile menu)
      def render_auth_actions
        div(class: 'nav__user-menu hidden md:block') do
          if authenticated?
            render_profile_menu
          else
            link_to('Login', '/login', class: 'nav__link text-sm font-medium')
          end
        end

        # Mobile login link
        return if authenticated?

        div(class: 'md:hidden') do
          link_to('Login', '/login', class: 'nav__link text-sm font-medium')
        end
      end

      # Render profile dropdown menu for authenticated users
      def render_profile_menu
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

      # Render admin menu item if user is administrator
      def render_admin_menu_item
        render RubyUI::DropdownMenuItem.new(href: admin_root_path) { 'Administration' }
      end

      # Render logout menu item with link using Turbo method
      def render_logout_menu_item
        render RubyUI::DropdownMenuItem.new(
          href: '/logout',
          data: { turbo_method: :post }
        ) { 'Logout' }
      end

      # Get current user name for display
      def current_user_name
        user = @current_user
        user&.name || 'Account'
      end

      # Check if current user is an administrator
      def user_is_admin?
        user = @current_user
        user&.administrator? || false
      end
    end
  end
end
